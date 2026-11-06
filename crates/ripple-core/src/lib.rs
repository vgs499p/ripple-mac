// SPDX-License-Identifier: GPL-2.0-or-later
//! Portable, direct-to-Ripple web client. No GUI, browser engine or proxy server.
mod account;
mod parse;
mod session;
mod user_agent;
pub use account::{Choice, EditableForm, Field, FieldKind, Notification};
pub use session::SessionStorage;
pub use user_agent::user_agent;

use futures_util::StreamExt;
pub use parse::{parse_page, Announcement, Composer, Link, Page, Person, Post, Profile};
use reqwest::{redirect::Policy, Client as HttpClient, Response};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use url::Url;

pub const ORIGIN: &str = "https://ripple.tel/";
const HTML_LIMIT: usize = 4 * 1024 * 1024;
const IMAGE_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not reach Ripple. Check your connection and try again.")]
    Network(#[from] reqwest::Error),
    #[error("Ripple returned HTTP {0}.")]
    Http(u16),
    #[error("Ripple returned an unexpected page. Its layout may have changed.")]
    UnexpectedPage,
    #[error("This request is not a supported Ripple address.")]
    InvalidAddress,
    #[error("The response exceeded the prototype's size limit.")]
    TooLarge,
    #[error("Sign-in was not confirmed. Check your details, or use the website if a browser challenge is required.")]
    LoginFailed,
    #[error("Sign in first. This page does not contain a supported post form.")]
    NoComposer,
    #[error("Write a post between 1 and 256 characters.")]
    InvalidPost,
    #[error("Ripple did not confirm the post. Check the website before trying again to avoid duplicates.")]
    UnconfirmedPost,
    #[error("Ripple did not confirm the action. Refresh before trying again.")]
    UnconfirmedAction,
    #[error("This page does not expose a supported image upload field.")]
    NoUpload,
    #[error(
        "Choose up to five PNG, JPEG, GIF or WebP images, at most 8 MiB each and 20 MiB total."
    )]
    InvalidUpload,
    #[error(
        "The settings form changed or contains an invalid value. Refresh and review the form."
    )]
    InvalidForm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Route {
    Public,
    Following,
    Notifications,
    Users,
    Profile(String),
    Search(String),
    Thread(u64),
    Page(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Composition {
    Post,
    Reply(u64),
    Quote(u64),
}

#[derive(Clone)]
pub struct Upload {
    pub name: String,
    pub bytes: Vec<u8>,
    pub mime: String,
}

impl std::fmt::Debug for Upload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Upload")
            .field("name", &self.name)
            .field("size", &self.bytes.len())
            .finish()
    }
}

impl Route {
    pub fn url(&self, page: u32) -> Result<Url, Error> {
        let mut url = Url::parse(ORIGIN).unwrap();
        match self {
            Self::Page(address) => {
                let candidate = Url::parse(address).map_err(|_| Error::InvalidAddress)?;
                if !trusted_url(&candidate) {
                    return Err(Error::InvalidAddress);
                }
                url = candidate;
            }
            Self::Public => (),
            Self::Following => {
                url.query_pairs_mut().append_pair("tlmode", "following");
            }
            Self::Notifications => {
                url.query_pairs_mut().append_pair("tlmode", "mentions");
            }
            Self::Users => url.set_path("/users"),
            Self::Profile(name) => {
                let name = name.trim_start_matches('@');
                if name.is_empty()
                    || name == "."
                    || name == ".."
                    || name.len() > 64
                    || !name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c))
                {
                    return Err(Error::InvalidAddress);
                }
                // The ~ alias currently redirects to HTTP. Use the observed HTTPS canonical route.
                url.set_path(&format!("/{name}"));
            }
            Self::Search(query) => {
                url.set_path("/results");
                url.query_pairs_mut().append_pair("q", query);
            }
            Self::Thread(id) => url.set_path(&format!("/status/{id}")),
        }
        if page > 1 {
            url.query_pairs_mut()
                .append_pair("pageno", &page.to_string());
        }
        Ok(url)
    }

    /// Maps the routes observed in Ripple's navigation, including its timeline tabs.
    pub fn from_navigation(address: &str) -> Self {
        if let Ok(url) = Url::parse(address) {
            if trusted_url(&url) {
                if url.path() == "/users" {
                    return Self::Users;
                }
                if url.path() == "/" {
                    match url
                        .query_pairs()
                        .find(|(key, _)| key == "tlmode")
                        .map(|(_, value)| value.into_owned())
                        .as_deref()
                    {
                        Some("following") => return Self::Following,
                        Some("mentions") => return Self::Notifications,
                        _ => (),
                    }
                }
            }
        }
        Self::Page(address.to_string())
    }
}

#[derive(Clone)]
pub struct Client {
    http: HttpClient,
    media: HttpClient,
    media_slots: Arc<Semaphore>,
    navigation: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    actions: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    session: Arc<session::Session>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RippleClient { session: <private> }")
    }
}

fn redirect_policy() -> Policy {
    Policy::custom(|attempt| {
        if attempt.previous().len() >= 5 {
            attempt.error("Too many redirects")
        } else if trusted_url(attempt.url()) {
            attempt.follow()
        } else {
            attempt.error("Redirect left Ripple")
        }
    })
}

pub fn trusted_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("ripple.tel")
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        Self::with_session(Arc::new(session::Session::default()))
    }

    pub fn restore_session(storage: Arc<dyn SessionStorage>) -> Result<Self, Error> {
        Self::with_session(Arc::new(session::Session::restore(storage)))
    }

    pub fn remember_session(
        &self,
        storage: Arc<dyn SessionStorage>,
        account: String,
    ) -> Result<(), String> {
        self.session.remember(storage, account)
    }

    pub fn forget_session(&self) -> Result<(), String> {
        self.session.forget()
    }

    pub fn saved_account(&self) -> Option<String> {
        self.session.account()
    }

    pub fn take_session_error(&self) -> Option<String> {
        self.session.take_error()
    }

    fn with_session(session: Arc<session::Session>) -> Result<Self, Error> {
        let builder = || {
            HttpClient::builder()
                .user_agent(user_agent())
                .https_only(true)
                .connect_timeout(Duration::from_secs(8))
                .timeout(Duration::from_secs(20))
                .redirect(redirect_policy())
                .retry(reqwest::retry::never())
        };
        Ok(Self {
            http: builder().cookie_provider(session.clone()).build()?,
            media: builder().build()?,
            media_slots: Arc::new(Semaphore::new(4)),
            navigation: Arc::default(),
            actions: Arc::default(),
            session,
        })
    }

    pub async fn load(&self, route: &Route, page: u32) -> Result<Page, Error> {
        if let Route::Page(address) = route {
            if !self.navigation.lock().unwrap().contains(address) {
                return Err(Error::InvalidAddress);
            }
        }
        let response = self.http.get(route.url(page)?).send().await?;
        let fetched = page_response(response).await?;
        self.navigation
            .lock()
            .unwrap()
            .extend(fetched.navigation.iter().map(|l| l.url.clone()));
        let mut actions = self.actions.lock().unwrap();
        for post in &fetched.posts {
            actions.extend(
                post.like_action
                    .iter()
                    .chain(post.repost_action.iter())
                    .cloned(),
            );
        }
        if let Some(profile) = &fetched.profile {
            actions.extend(profile.follow_action.iter().cloned());
        }
        Ok(fetched)
    }

    pub async fn save_settings(
        &self,
        route: Route,
        action: String,
        values: Vec<(String, String)>,
        files: Vec<(String, Upload)>,
    ) -> Result<(Page, bool), Error> {
        let before = self.load(&route, 1).await?;
        let matching_forms: Vec<_> = before
            .editable_forms
            .iter()
            .filter(|f| f.form.action == action)
            .collect();
        if matching_forms.len() != 1 {
            return Err(Error::InvalidForm);
        }
        let form = matching_forms[0];
        account::validate_submission(form, &values, &files)?;
        let has_uploads = !files.is_empty();
        let changes_password = form.fields.iter().any(|f| {
            f.kind == FieldKind::Password
                && values
                    .iter()
                    .any(|(name, value)| name == &f.name && !value.is_empty())
        });
        let mut fields = form.form.hidden.clone();
        fields.extend(values.clone());
        if let Some(submit) = &form.form.submit {
            fields.push(submit.clone());
        }
        let mut request = self
            .http
            .post(&action)
            .header("Origin", "https://ripple.tel")
            .header("Referer", route.url(1)?.as_str());
        request = if form.form.multipart {
            let mut multipart = fields
                .into_iter()
                .fold(reqwest::multipart::Form::new(), |f, (k, v)| f.text(k, v));
            for (name, file) in files {
                multipart = multipart.part(
                    name,
                    reqwest::multipart::Part::bytes(file.bytes)
                        .file_name(file.name)
                        .mime_str(&file.mime)
                        .map_err(|_| Error::InvalidUpload)?,
                );
            }
            request.multipart(multipart)
        } else {
            request.form(&fields)
        };
        let response = request.send().await.map_err(|_| Error::UnconfirmedAction)?;
        bounded_body(response, HTML_LIMIT)
            .await
            .map_err(|_| Error::UnconfirmedAction)?;
        let after = self
            .load(&route, 1)
            .await
            .map_err(|_| Error::UnconfirmedAction)?;
        let confirmed = !has_uploads
            && !changes_password
            && after
                .editable_forms
                .iter()
                .find(|f| f.form.action == action)
                .is_some_and(|f| {
                    f.fields.iter().all(|field| {
                        if matches!(field.kind, FieldKind::File | FieldKind::Password) {
                            return true;
                        }
                        if field.kind == FieldKind::Checkbox {
                            return field.checked
                                == values.iter().any(|(name, _)| name == &field.name);
                        }
                        values
                            .iter()
                            .find(|(name, _)| name == &field.name)
                            .is_some_and(|(_, v)| v == &field.value)
                    })
                });
        Ok((after, confirmed))
    }

    pub async fn image(&self, address: &str) -> Result<Vec<u8>, Error> {
        let url = Url::parse(address).map_err(|_| Error::InvalidAddress)?;
        if !trusted_url(&url) {
            return Err(Error::InvalidAddress);
        }
        let _permit = self
            .media_slots
            .acquire()
            .await
            .expect("semaphore stays open");
        bounded_body(self.media.get(url).send().await?, IMAGE_LIMIT).await
    }

    /// Invokes only a same-origin action link discovered in the current HTML.
    pub async fn social_action(&self, address: &str) -> Result<(), Error> {
        let url = Url::parse(address).map_err(|_| Error::InvalidAddress)?;
        if !trusted_url(&url) || !self.actions.lock().unwrap().contains(url.as_str()) {
            return Err(Error::InvalidAddress);
        }
        let response = self
            .http
            .get(url)
            .header("Referer", ORIGIN)
            .send()
            .await
            .map_err(|_| Error::UnconfirmedAction)?;
        bounded_body(response, HTML_LIMIT)
            .await
            .map_err(|_| Error::UnconfirmedAction)?;
        Ok(())
    }

    /// Passwords are never saved. The caller may remember a confirmed session in a secure store.
    pub async fn login(&self, username: String, password: String) -> Result<Page, Error> {
        let home = self.load(&Route::Public, 1).await?;
        let form = home.login_form.ok_or(Error::UnexpectedPage)?;
        let mut fields = form.hidden;
        fields.push(("username".into(), username));
        fields.push(("password".into(), password));
        let response = self
            .http
            .post(form.action)
            .header("Origin", "https://ripple.tel")
            .header("Referer", ORIGIN)
            .form(&fields)
            .send()
            .await?;
        let _ = bounded_body(response, HTML_LIMIT).await?;
        let page = self.load(&Route::Public, 1).await?;
        if page.composer.is_none() || page.login_form.is_some() {
            return Err(Error::LoginFailed);
        }
        Ok(page)
    }

    /// Submits only to a discovered form. Requests are never automatically retried.
    pub async fn publish(&self, contents: String) -> Result<Page, Error> {
        self.publish_composition(contents, Composition::Post, Vec::new(), None)
            .await
    }

    pub async fn publish_composition(
        &self,
        contents: String,
        composition: Composition,
        uploads: Vec<Upload>,
        author: Option<String>,
    ) -> Result<Page, Error> {
        if (contents.trim().is_empty() && uploads.is_empty())
            || contents.encode_utf16().count() > 256
        {
            return Err(Error::InvalidPost);
        }
        let before = self.load(&Route::Public, 1).await?;
        let target = match composition {
            Composition::Reply(id) => Route::Thread(id),
            _ => Route::Public,
        };
        let context = if target == Route::Public {
            before.clone()
        } else {
            self.load(&target, 1).await?
        };
        let baseline_ids: std::collections::HashSet<u64> =
            context.posts.iter().map(|post| post.id).collect();
        let composer = match composition {
            Composition::Quote(_) => context.quote_form,
            _ => context.composer,
        }
        .ok_or(Error::NoComposer)?;
        if uploads.len() > 5
            || uploads.iter().map(|u| u.bytes.len()).sum::<usize>() > 20 * 1024 * 1024
            || uploads.iter().any(|u| {
                u.bytes.len() > IMAGE_LIMIT
                    || !matches!(
                        u.mime.as_str(),
                        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
                    )
            })
        {
            return Err(Error::InvalidUpload);
        }
        if !uploads.is_empty() && (composer.file_field.is_none() || !composer.form.multipart) {
            return Err(Error::NoUpload);
        }
        let mut fields = composer.form.hidden;
        if let Composition::Quote(id) = composition {
            let Some(field) = fields.iter_mut().find(|(key, _)| key == "qid") else {
                return Err(Error::NoComposer);
            };
            field.1 = id.to_string();
        }
        fields.push((composer.field, contents.clone()));
        if let Some(submit) = composer.form.submit {
            fields.push(submit);
        }
        let mut request = self
            .http
            .post(&composer.form.action)
            .header("Origin", "https://ripple.tel")
            .header("Referer", ORIGIN);
        request = if composer.form.multipart {
            let mut form = fields
                .into_iter()
                .fold(reqwest::multipart::Form::new(), |f, (k, v)| f.text(k, v));
            for upload in uploads {
                let part = reqwest::multipart::Part::bytes(upload.bytes)
                    .file_name(upload.name)
                    .mime_str(&upload.mime)
                    .map_err(|_| Error::InvalidUpload)?;
                form = form.part(
                    composer.file_field.as_ref().ok_or(Error::NoUpload)?.clone(),
                    part,
                );
            }
            request.multipart(form)
        } else {
            request.form(&fields)
        };
        let response = request.send().await.map_err(|_| Error::UnconfirmedPost)?;
        let _ = bounded_body(response, HTML_LIMIT)
            .await
            .map_err(|_| Error::UnconfirmedPost)?;
        // Replies may only appear inside their conversation, not in the first
        // page of the public timeline. Verify against the same route/form page
        // used to compose so a successful reply is not reported as a failure.
        let after = self
            .load(&target, 1)
            .await
            .map_err(|_| Error::UnconfirmedPost)?;
        let confirmed = after.posts.iter().any(|p| {
            !baseline_ids.contains(&p.id)
                && parse::normalize(&p.text).contains(&parse::normalize(&contents))
                && (!contents.trim().is_empty() || !p.images.is_empty())
                && author.as_ref().is_none_or(|a| {
                    p.handle
                        .trim_start_matches('@')
                        .eq_ignore_ascii_case(a.trim_start_matches('@'))
                })
        });
        if !confirmed {
            return Err(Error::UnconfirmedPost);
        }
        Ok(after)
    }
}

async fn page_response(response: Response) -> Result<Page, Error> {
    let final_url = response.url().clone();
    let bytes = bounded_body(response, HTML_LIMIT).await?;
    parse_page(&String::from_utf8_lossy(&bytes), &final_url)
}

async fn bounded_body(response: Response, limit: usize) -> Result<Vec<u8>, Error> {
    if !response.status().is_success() {
        return Err(Error::Http(response.status().as_u16()));
    }
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(Error::TooLarge);
    }
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if body.len() + chunk.len() > limit {
            return Err(Error::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
