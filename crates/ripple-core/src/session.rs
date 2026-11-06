// SPDX-License-Identifier: GPL-2.0-or-later
//! Opaque secure-store persistence. No filesystem or platform storage in the web core.
use crate::trusted_url;
use cookie_store::{Cookie, CookieDomain, CookieStore};
use reqwest::{cookie::CookieStore as HttpCookieStore, header::HeaderValue};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use url::Url;

pub trait SessionStorage: Send + Sync {
    fn load(&self) -> Result<Option<Vec<u8>>, String>;
    fn save(&self, data: &[u8]) -> Result<(), String>;
    fn clear(&self) -> Result<(), String>;
}

#[derive(Serialize, Deserialize)]
struct SavedSession {
    version: u8,
    account: String,
    cookies: Vec<Cookie<'static>>,
}

#[derive(Default)]
struct State {
    cookies: CookieStore,
    storage: Option<Arc<dyn SessionStorage>>,
    account: Option<String>,
    last_saved: Vec<u8>,
    error: Option<String>,
    revoked: bool,
}

#[derive(Default)]
pub(crate) struct Session(Mutex<State>);

fn allowed(cookie: &Cookie<'_>) -> bool {
    matches!(&cookie.domain, CookieDomain::HostOnly(d) | CookieDomain::Suffix(d) if d == "ripple.tel")
}

impl State {
    fn persist(&mut self) -> Result<(), String> {
        let (Some(storage), Some(account)) = (&self.storage, &self.account) else {
            return Ok(());
        };
        let data = serde_json::to_vec(&SavedSession {
            version: 1,
            account: account.clone(),
            // Include browser session cookies too; retain absolute server expiration dates.
            cookies: self
                .cookies
                .iter_unexpired()
                .filter(|c| allowed(c))
                .cloned()
                .collect(),
        })
        .map_err(|_| "Could not encode the saved session.".to_string())?;
        if data != self.last_saved {
            storage.save(&data)?;
            self.last_saved = data;
        }
        Ok(())
    }
}

impl Session {
    pub fn restore(storage: Arc<dyn SessionStorage>) -> Self {
        let mut state = State::default();
        match storage.load() {
            Ok(Some(data)) => {
                let decoded = (data.len() <= 256 * 1024)
                    .then(|| serde_json::from_slice::<SavedSession>(&data).ok())
                    .flatten();
                match decoded {
                    Some(saved)
                        if saved.version == 1
                            && !saved.account.is_empty()
                            && saved.account.len() <= 65 =>
                    {
                        let cookies = saved
                            .cookies
                            .into_iter()
                            .filter(allowed)
                            .map(Ok::<_, cookie_store::Error>);
                        if let Ok(store) = CookieStore::from_cookies(cookies, false) {
                            if store.iter_unexpired().next().is_some() {
                                state.cookies = store;
                                state.account = Some(saved.account);
                                state.storage = Some(storage.clone());
                            }
                        }
                    }
                    _ => {
                        state.error = Some("The saved sign-in is unreadable. Sign in again.".into())
                    }
                }
            }
            Ok(None) => (),
            Err(error) => state.error = Some(error),
        }
        Self(Mutex::new(state))
    }

    pub fn remember(
        &self,
        storage: Arc<dyn SessionStorage>,
        account: String,
    ) -> Result<(), String> {
        let mut state = self.0.lock().unwrap();
        if state.revoked {
            return Err("This session was signed out.".into());
        }
        state.storage = Some(storage);
        state.account = Some(account);
        state.last_saved.clear();
        state.persist()
    }

    pub fn forget(&self) -> Result<(), String> {
        let mut state = self.0.lock().unwrap();
        // Delete first. On a Keychain error, report sign-out failure so the user can retry.
        if let Some(storage) = &state.storage {
            storage.clear()?;
        }
        state.revoked = true;
        state.cookies.clear();
        state.storage = None;
        state.account = None;
        state.last_saved.clear();
        Ok(())
    }

    pub fn account(&self) -> Option<String> {
        self.0.lock().unwrap().account.clone()
    }
    pub fn take_error(&self) -> Option<String> {
        self.0.lock().unwrap().error.take()
    }
}

impl HttpCookieStore for Session {
    fn set_cookies(&self, headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        if !trusted_url(url) {
            return;
        }
        let mut state = self.0.lock().unwrap();
        if state.revoked {
            return;
        }
        for value in headers.filter_map(|h| h.to_str().ok()) {
            let _ = state.cookies.parse(value, url);
        }
        if let Err(error) = state.persist() {
            state.error = Some(error);
        }
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        if !trusted_url(url) {
            return None;
        }
        let state = self.0.lock().unwrap();
        if state.revoked {
            return None;
        }
        let text = state
            .cookies
            .get_request_values(url)
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        if text.is_empty() {
            None
        } else {
            HeaderValue::from_str(&text).ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ORIGIN;
    #[derive(Default)]
    struct MemoryStore {
        data: Mutex<Option<Vec<u8>>>,
        writes: Mutex<usize>,
    }
    impl SessionStorage for MemoryStore {
        fn load(&self) -> Result<Option<Vec<u8>>, String> {
            Ok(self.data.lock().unwrap().clone())
        }
        fn save(&self, data: &[u8]) -> Result<(), String> {
            *self.data.lock().unwrap() = Some(data.to_vec());
            *self.writes.lock().unwrap() += 1;
            Ok(())
        }
        fn clear(&self) -> Result<(), String> {
            *self.data.lock().unwrap() = None;
            Ok(())
        }
    }
    fn put(session: &Session, value: &str) {
        session.set_cookies(
            &mut [HeaderValue::from_str(value).unwrap()].iter(),
            &Url::parse(ORIGIN).unwrap(),
        );
    }
    #[test]
    fn restart_rotation_expiration_and_signout() {
        let storage = Arc::new(MemoryStore::default());
        let first = Session::default();
        put(&first, "session=one; Path=/; Secure; HttpOnly");
        put(&first, "dead=no; Max-Age=0; Path=/");
        first.remember(storage.clone(), "@alice".into()).unwrap();
        let second = Session::restore(storage.clone());
        assert_eq!(second.account().as_deref(), Some("@alice"));
        assert_eq!(
            second.cookies(&Url::parse(ORIGIN).unwrap()).unwrap(),
            "session=one"
        );
        put(&second, "session=two; Path=/; Secure; HttpOnly");
        let writes = *storage.writes.lock().unwrap();
        put(&second, "session=two; Path=/; Secure; HttpOnly");
        assert_eq!(*storage.writes.lock().unwrap(), writes);
        let third = Session::restore(storage.clone());
        assert_eq!(
            third.cookies(&Url::parse(ORIGIN).unwrap()).unwrap(),
            "session=two"
        );
        second.forget().unwrap();
        put(&second, "session=late-response; Path=/");
        assert!(storage.load().unwrap().is_none());
        assert!(Session::restore(storage).account().is_none());
    }
    #[test]
    fn scope_and_path_are_preserved() {
        let session = Session::default();
        put(&session, "scoped=secret; Path=/account; Secure");
        put(&session, "foreign=secret; Domain=example.com; Path=/");
        assert!(session.cookies(&Url::parse(ORIGIN).unwrap()).is_none());
        assert!(session
            .cookies(&Url::parse("https://example.com/account").unwrap())
            .is_none());
        assert!(session
            .cookies(&Url::parse("http://ripple.tel/account").unwrap())
            .is_none());
        assert_eq!(
            session
                .cookies(&Url::parse("https://ripple.tel/account").unwrap())
                .unwrap(),
            "scoped=secret"
        );
    }
    #[test]
    fn unreadable_store_does_not_break_browsing() {
        let storage = Arc::new(MemoryStore::default());
        storage.save(b"invalid data").unwrap();
        let session = Session::restore(storage);
        assert!(session.account().is_none());
        assert!(session.take_error().is_some());
    }
    #[test]
    fn failed_signout_does_not_claim_the_saved_session_was_removed() {
        struct LockedStore(MemoryStore);
        impl SessionStorage for LockedStore {
            fn load(&self) -> Result<Option<Vec<u8>>, String> {
                self.0.load()
            }
            fn save(&self, data: &[u8]) -> Result<(), String> {
                self.0.save(data)
            }
            fn clear(&self) -> Result<(), String> {
                Err("Keychain is locked".into())
            }
        }
        let storage = Arc::new(LockedStore(MemoryStore::default()));
        let session = Session::default();
        put(&session, "session=synthetic; Path=/; Secure");
        session.remember(storage.clone(), "@alice".into()).unwrap();
        assert!(session.forget().is_err());
        assert!(session.account().is_some());
        assert!(storage.load().unwrap().is_some());
    }
}
