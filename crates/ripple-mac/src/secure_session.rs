// SPDX-License-Identifier: GPL-2.0-or-later
use ripple_core::Client;

#[cfg(target_os = "macos")]
mod keychain {
    use ripple_core::SessionStorage;
    use security_framework::passwords::{
        delete_generic_password, get_generic_password, set_generic_password,
    };
    pub struct Keychain(String);
    impl Default for Keychain {
        fn default() -> Self {
            Self(SERVICE.into())
        }
    }
    const SERVICE: &str = "tel.ripple.native.beta.session";
    const ACCOUNT: &str = "ripple.tel";
    impl SessionStorage for Keychain {
        fn load(&self) -> Result<Option<Vec<u8>>, String> {
            match get_generic_password(&self.0, ACCOUNT) {
                Ok(data) => Ok(Some(data)),
                Err(error) if error.code() == -25300 => Ok(None),
                Err(_) => Err(
                    "Could not read your saved sign-in. Unlock the login Keychain and try again."
                        .into(),
                ),
            }
        }
        fn save(&self, data: &[u8]) -> Result<(), String> {
            set_generic_password(&self.0, ACCOUNT, data).map_err(|_| "Signed in, but the session could not be saved to Keychain. Unlock it and sign in again to keep your login.".into())
        }
        fn clear(&self) -> Result<(), String> {
            match delete_generic_password(&self.0, ACCOUNT) {
                Ok(()) => Ok(()),
                Err(error) if error.code() == -25300 => Ok(()),
                Err(_) => Err("Could not sign out because the saved session could not be removed. Unlock Keychain and try Sign out again.".into()),
            }
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        #[ignore = "Uses a dedicated synthetic item in the real macOS Keychain"]
        fn keychain_round_trip() {
            let store = Keychain(format!(
                "tel.ripple.native.beta.session.selftest.{}",
                std::process::id()
            ));
            store.save(b"synthetic session").unwrap();
            assert_eq!(store.load().unwrap().unwrap(), b"synthetic session");
            store.save(b"rotated synthetic session").unwrap();
            assert_eq!(store.load().unwrap().unwrap(), b"rotated synthetic session");
            store.clear().unwrap();
            assert!(store.load().unwrap().is_none());
        }
    }
}

pub fn restore() -> Client {
    #[cfg(target_os = "macos")]
    {
        Client::restore_session(std::sync::Arc::new(keychain::Keychain::default()))
            .expect("TLS client initialization")
    }
    #[cfg(not(target_os = "macos"))]
    {
        Client::new().expect("TLS client initialization")
    }
}

pub fn remember(client: &Client, account: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        client.remember_session(std::sync::Arc::new(keychain::Keychain::default()), account)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (client, account);
        Err("Secure session storage is not configured on this platform.".into())
    }
}

pub fn description() -> &'static str {
    if cfg!(target_os = "macos") {
        "Your session is saved in Keychain and restored when you reopen Ripple. Sign out removes it. Your password is never stored."
    } else {
        "Preferences save automatically. Sign-in lasts until the app closes; secure session persistence is not available on this platform yet. Your password is never stored."
    }
}
