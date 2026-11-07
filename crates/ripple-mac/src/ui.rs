// SPDX-License-Identifier: GPL-2.0-or-later
#[path = "settings_view.rs"]
mod settings_view;
use crate::preferences::{self, Appearance, Density, Preferences};
use crate::themes;
use iced::{
    alignment, border, font, keyboard,
    widget::{
        button, checkbox, column, container, image, pick_list, row, scrollable, stack, text,
        text_editor, text_input, toggler, Column, Row, Space,
    },
    window, Background, Border, Color, Element, Font, Length, Size, Subscription, Task, Theme,
};
use ripple_core::{Client, Composition, FieldKind, Link, Page, Post, Route, Upload};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::Cursor,
    sync::Arc,
};
use url::Url;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}
const BOLD: Font = Font {
    family: font::Family::Name("Helvetica"),
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPage {
    Appearance,
    General,
    About,
    License,
}

#[derive(Debug, Clone)]
pub enum Message {
    #[cfg(target_os = "macos")]
    InstallMacMenu,
    RemoveTheme,
    ThemeSelected(themes::Choice),
    ImportTheme,
    ThemeImported(Result<Option<themes::ThemePack>, String>),
    ExportTheme,
    ThemeExported(Result<bool, String>),
    Local(LocalPage),
    CloseLocal,
    Appearance(Appearance),
    Accent([u8; 3]),
    AccentText(String),
    ApplyAccent,
    Density(Density),
    TextSize(u16),
    Wallpapers(bool),
    ResetPreferences,
    PreferencesSaved(u64, Result<(), String>),
    SystemAppearance(bool),
    DetectAppearance,
    BrandReady(u64, Result<(image::Handle, image::Handle), String>),
    Navigate(Route),
    Back,
    Refresh,
    Older,
    Newer,
    SearchChanged(String),
    Search,
    Loaded {
        id: u64,
        page: u32,
        result: Result<Page, String>,
    },
    ImageLoaded(String, Result<image::Handle, String>),
    LoadImage(String),
    LowData(bool),
    Open(String),
    ShowLogin,
    CloseLogin,
    Username(String),
    Password(String),
    Login,
    LoggedIn {
        id: u64,
        client: Client,
        result: Result<Page, String>,
    },
    SessionRestored(Client),
    SignOut,
    Edit(text_editor::Action),
    Publish,
    DiscardDraft,
    Published {
        id: u64,
        result: Result<Page, String>,
    },
    Dismiss,
    Resize(Size),
    FocusSearch,
    Quit,
    People,
    DirectoryPage(u32),
    Trends,
    BrowserOpened(Result<(), String>),
    ProfileTab(u8),
    Social(String),
    SocialDone {
        id: u64,
        result: Result<Page, String>,
    },
    Compose(Composition),
    PickPhotos,
    PhotosPicked(Result<Vec<Upload>, String>),
    RemovePhoto(usize),
    Portal(String),
    FieldText(usize, usize, String),
    FieldCheck(usize, usize, bool),
    FieldEdit(usize, usize, text_editor::Action),
    FormPhoto(usize, usize),
    FormPhotos(usize, usize, Result<Vec<Upload>, String>),
    SaveForm(usize),
    SettingsSaved {
        id: u64,
        result: Result<(Page, bool), String>,
    },
    #[cfg(feature = "qa")]
    PreviewTheme,
    #[cfg(feature = "qa")]
    Capture,
    #[cfg(feature = "qa")]
    BenchStart,
    #[cfg(feature = "qa")]
    BenchFrame(std::time::Instant),
    #[cfg(feature = "qa")]
    Captured(window::Screenshot),
}

#[derive(Debug)]
enum Media {
    Loading,
    Ready(image::Handle),
    Failed,
}

struct FieldDraft {
    value: String,
    checked: bool,
    editor: text_editor::Content,
    uploads: Vec<Upload>,
}

pub struct App {
    cached_theme: Theme,
    preferences: Preferences,
    local_page: Option<LocalPage>,
    accent_text: String,
    system_dark: bool,
    preference_revision: u64,
    brand_revision: u64,
    settings_path: std::path::PathBuf,
    preference_error: Option<String>,
    save_order: Arc<std::sync::Mutex<u64>>,

    client: Client,
    route: Route,
    page: Option<Page>,
    page_number: u32,
    generation: u64,
    loading: bool,
    error: Option<String>,
    notice: Option<String>,
    search: String,
    history: Vec<Route>,
    images: HashMap<String, Media>,
    low_data: bool,
    width: f32,
    login_open: bool,
    username: String,
    password: String,
    account: Option<String>,
    signing_in: bool,
    publishing: bool,
    draft: text_editor::Content,
    snapshot: bool,
    directory: bool,
    trend_view: bool,
    profile_tab: u8,
    media_revision: u64,
    navigation: Vec<Link>,
    form_drafts: Vec<Vec<FieldDraft>>,
    heart: image::Handle,
    comment: image::Handle,
    repost: image::Handle,
    logo: image::Handle,
    header: image::Handle,
    quote: image::Handle,
    composition: Composition,
    uploads: Vec<Upload>,
    image_order: VecDeque<String>,
    work_slots: Arc<tokio::sync::Semaphore>,
    #[cfg(feature = "qa")]
    bench: Vec<std::time::Instant>,
    #[cfg(feature = "qa")]
    bench_remaining: usize,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let settings_path = preferences::settings_path();
        let loaded = Preferences::load(&settings_path);
        let preference_error = loaded.as_ref().err().cloned();
        let preferences = loaded.unwrap_or_default();
        let cached_theme = themes::build(&preferences, false);
        let mut app = Self {
            cached_theme,
            accent_text: preferences::hex(preferences.accent),
            low_data: !preferences.load_photos,
            preferences,
            settings_path,
            preference_error,
            local_page: None,
            system_dark: false,
            preference_revision: 0,
            brand_revision: 0,
            save_order: Arc::new(std::sync::Mutex::new(0)),
            client: Client::new().expect("TLS client initialization"),
            route: Route::Public,
            page: None,
            page_number: 1,
            generation: 0,
            loading: false,
            error: None,
            notice: None,
            search: String::new(),
            history: Vec::new(),
            images: HashMap::new(),
            width: 1180.0,
            login_open: false,
            username: String::new(),
            password: String::new(),
            account: None,
            signing_in: false,
            publishing: false,
            draft: text_editor::Content::new(),
            snapshot: false,
            directory: false,
            trend_view: false,
            profile_tab: 0,
            media_revision: 0,
            navigation: Vec::new(),
            form_drafts: Vec::new(),
            heart: image::Handle::from_bytes(
                include_bytes!("../../../assets/ui/heart.png").as_slice(),
            ),
            comment: image::Handle::from_bytes(
                include_bytes!("../../../assets/ui/comment.png").as_slice(),
            ),
            repost: image::Handle::from_bytes(
                include_bytes!("../../../assets/ui/repost.png").as_slice(),
            ),
            quote: image::Handle::from_bytes(
                include_bytes!("../../../assets/ui/quote.png").as_slice(),
            ),
            logo: image::Handle::from_bytes(
                include_bytes!("../../../assets/ui/logo.png").as_slice(),
            ),
            header: image::Handle::from_bytes(
                include_bytes!("../../../assets/ui/header.png").as_slice(),
            ),
            composition: Composition::Post,
            uploads: Vec::new(),
            image_order: VecDeque::new(),
            work_slots: Arc::new(tokio::sync::Semaphore::new(4)),
            #[cfg(feature = "qa")]
            bench: Vec::new(),
            #[cfg(feature = "qa")]
            bench_remaining: 0,
        };
        app.loading = true;
        let task = Task::perform(
            async {
                tokio::task::spawn_blocking(crate::secure_session::restore)
                    .await
                    .expect("session restoration worker")
            },
            Message::SessionRestored,
        );
        let brand = app.update_brand();
        (
            app,
            Task::batch([
                task,
                brand,
                #[cfg(target_os = "macos")]
                window::get_latest().map(|_| Message::InstallMacMenu),
                Task::perform(
                    async {
                        tokio::task::spawn_blocking(preferences::system_is_dark)
                            .await
                            .unwrap_or(false)
                    },
                    Message::SystemAppearance,
                ),
            ]),
        )
    }

    fn load(&mut self, route: Route, page: u32) -> Task<Message> {
        self.generation += 1;
        let id = self.generation;
        if self.route != route {
            self.page = None;
            self.snapshot = false;
            self.page_number = 1;
        }
        self.route = route.clone();
        self.loading = true;
        self.error = None;
        self.login_open = false;
        self.directory = route == Route::Users;
        self.trend_view = false;
        self.profile_tab = 0;
        let client = self.client.clone();
        Task::perform(
            async move { client.load(&route, page).await.map_err(|e| e.to_string()) },
            move |result| Message::Loaded { id, page, result },
        )
    }

    fn media_task(&mut self, url: String) -> Task<Message> {
        if matches!(
            self.images.get(&url),
            Some(Media::Loading | Media::Ready(_))
        ) {
            return Task::none();
        }
        self.images.insert(url.clone(), Media::Loading);
        let client = self.client.clone();
        let bundled = self
            .snapshot
            .then(|| crate::snapshot_media::get(&url))
            .flatten();
        let slots = self.work_slots.clone();
        Task::perform(
            async move {
                let result = async {
                    let _permit = slots.acquire().await.map_err(|e| e.to_string())?;
                    let bytes = if let Some(bytes) = bundled {
                        bytes.to_vec()
                    } else {
                        client.image(&url).await.map_err(|e| e.to_string())?
                    };
                    let avatar = url.contains("/pfp/");
                    tokio::task::spawn_blocking(move || {
                        let mut reader = ::image::ImageReader::new(Cursor::new(bytes))
                            .with_guessed_format()
                            .map_err(|e| e.to_string())?;
                        let mut limits = ::image::Limits::default();
                        limits.max_image_width = Some(8192);
                        limits.max_image_height = Some(8192);
                        limits.max_alloc = Some(64 * 1024 * 1024);
                        reader.limits(limits);
                        let decoded = reader.decode().map_err(|e| e.to_string())?;
                        let max = if avatar { 96 } else { 1000 };
                        let resized = decoded.thumbnail(max, max).into_rgba8();
                        Ok(image::Handle::from_rgba(
                            resized.width(),
                            resized.height(),
                            resized.into_raw(),
                        ))
                    })
                    .await
                    .map_err(|e| e.to_string())?
                }
                .await;
                (url, result)
            },
            |(url, result)| Message::ImageLoaded(url, result),
        )
    }

    fn load_visible_media(&mut self) -> Task<Message> {
        let mut urls = HashSet::new();
        if let Some(page) = &self.page {
            let mut photos = 0;
            for post in &page.posts {
                if let Some(avatar) = &post.avatar {
                    urls.insert(avatar.clone());
                }
                if !self.low_data {
                    for url in &post.images {
                        if photos < 24 {
                            urls.insert(url.clone());
                            photos += 1;
                        }
                    }
                }
            }
            if let Some(profile) = &page.profile {
                if let Some(avatar) = &profile.avatar {
                    urls.insert(avatar.clone());
                }
                if let Some(banner) = &profile.banner {
                    if !self.low_data {
                        urls.insert(banner.clone());
                    }
                }
                for person in profile
                    .followers
                    .iter()
                    .chain(profile.following.iter())
                    .take(60)
                {
                    if let Some(avatar) = &person.avatar {
                        urls.insert(avatar.clone());
                    }
                }
            }
            if let Some(background) = &page.background {
                if !self.low_data && self.preferences.wallpapers {
                    urls.insert(background.clone());
                }
            }
            for person in &page.people {
                if let Some(avatar) = &person.avatar {
                    urls.insert(avatar.clone());
                }
            }
        }
        // Bound decoded image memory to media referenced by the current capped feed.
        self.images.retain(|url, _| urls.contains(url));
        Task::batch(
            urls.into_iter()
                .map(|url| {
                    if self.snapshot && crate::snapshot_media::get(&url).is_none() {
                        self.images.entry(url).or_insert(Media::Failed);
                        Task::none()
                    } else {
                        self.media_task(url)
                    }
                })
                .collect::<Vec<_>>(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            #[cfg(target_os = "macos")]
            Message::InstallMacMenu => crate::mac_menu::install(),
            Message::RemoveTheme => {
                let id = self.preferences.theme.clone();
                self.preferences.imported_themes.retain(|p| p.id != id);
                return self.update(Message::ThemeSelected(themes::Choice {
                    id: "ripple".into(),
                    name: "Ripple".into(),
                }));
            }
            Message::ThemeSelected(choice) => {
                if !themes::choices(&self.preferences)
                    .iter()
                    .any(|c| c.id == choice.id)
                {
                    return Task::none();
                }
                self.preferences.theme = choice.id;
                self.preferences.accent = self
                    .preferences
                    .theme_pack()
                    .map(|p| p.colors.accent.0)
                    .unwrap_or([210, 4, 4]);
                self.accent_text = preferences::hex(self.preferences.accent);
                return Task::batch([self.save_preferences(), self.update_brand()]);
            }
            Message::ImportTheme => return Task::perform(import_theme(), Message::ThemeImported),
            Message::ThemeImported(result) => match result {
                Ok(Some(pack)) => {
                    let replacing = self
                        .preferences
                        .imported_themes
                        .iter()
                        .any(|p| p.id == pack.id);
                    if !replacing && self.preferences.imported_themes.len() >= 16 {
                        self.preference_error =
                            Some("Up to 16 custom themes can be installed.".into());
                        return Task::none();
                    }
                    let choice = themes::Choice {
                        id: pack.id.clone(),
                        name: pack.name.clone(),
                    };
                    self.preferences.imported_themes.retain(|p| p.id != pack.id);
                    self.preferences.imported_themes.push(pack);
                    return self.update(Message::ThemeSelected(choice));
                }
                Ok(None) => (),
                Err(error) => self.preference_error = Some(error),
            },
            Message::ExportTheme => {
                return Task::perform(
                    export_theme(themes::export(&self.preferences, self.system_dark)),
                    Message::ThemeExported,
                )
            }
            Message::ThemeExported(result) => {
                self.preference_error = result.err();
            }
            #[cfg(feature = "qa")]
            Message::BenchStart => {
                self.bench.clear();
                self.bench_remaining = 90;
                return scrollable::snap_to(
                    scrollable::Id::new("feed"),
                    scrollable::RelativeOffset::START,
                );
            }
            #[cfg(feature = "qa")]
            Message::BenchFrame(now) => {
                self.bench.push(now);
                self.bench_remaining = self.bench_remaining.saturating_sub(1);
                if self.bench_remaining > 0 {
                    return scrollable::scroll_by(
                        scrollable::Id::new("feed"),
                        scrollable::AbsoluteOffset { x: 0.0, y: 14.0 },
                    );
                }
                let mut durations: Vec<_> = self
                    .bench
                    .windows(2)
                    .skip(5)
                    .map(|w| w[1].duration_since(w[0]).as_secs_f64() * 1000.0)
                    .collect();
                durations.sort_by(f64::total_cmp);
                if !durations.is_empty() {
                    let result = format!(
                        "frames={} median_ms={:.2} p95_ms={:.2} max_ms={:.2}",
                        durations.len(),
                        durations[durations.len() / 2],
                        durations[durations.len() * 95 / 100],
                        durations[durations.len() - 1]
                    );
                    if let Ok(exe) = std::env::current_exe() {
                        if let Some(dir) = exe.ancestors().nth(4) {
                            let _ = std::fs::write(dir.join("scroll-benchmark.txt"), result);
                        }
                    }
                }
                return scrollable::snap_to(
                    scrollable::Id::new("feed"),
                    scrollable::RelativeOffset::START,
                );
            }
            #[cfg(feature = "qa")]
            Message::PreviewTheme => {
                self.preferences.appearance = Appearance::Dark;
                self.preferences.accent = [52, 122, 194];
                self.accent_text = preferences::hex(self.preferences.accent);
                return Task::batch([self.save_preferences(), self.update_brand()]);
            }
            #[cfg(feature = "qa")]
            Message::Capture => {
                return window::get_latest()
                    .and_then(window::screenshot)
                    .map(Message::Captured)
            }
            #[cfg(feature = "qa")]
            Message::Captured(shot) => {
                if let Ok(exe) = std::env::current_exe() {
                    if let Some(dir) = exe.ancestors().nth(4) {
                        let _ = ::image::save_buffer(
                            dir.join("ripple-preview.png"),
                            &shot.bytes,
                            shot.size.width,
                            shot.size.height,
                            ::image::ColorType::Rgba8,
                        );
                    }
                }
            }
            Message::Local(page) => {
                self.local_page = Some(page);
                self.login_open = false;
                self.password.clear();
            }
            Message::CloseLocal => {
                self.local_page = None;
            }
            Message::Appearance(mode) => {
                self.preferences.appearance = mode;
                return self.save_preferences();
            }
            Message::Accent(accent) => {
                self.preferences.accent = accent;
                self.accent_text = preferences::hex(accent);
                return Task::batch([self.save_preferences(), self.update_brand()]);
            }
            Message::AccentText(value) => self.accent_text = value,
            Message::ApplyAccent => {
                if let Some(accent) = preferences::parse_accent(&self.accent_text) {
                    return self.update(Message::Accent(accent));
                }
                self.preference_error =
                    Some("Enter a six-digit hex color, such as #D20404.".into());
            }
            Message::Density(density) => {
                self.preferences.density = density;
                return self.save_preferences();
            }
            Message::TextSize(size) => {
                self.preferences.text_size = size;
                self.preferences.validate();
                return self.save_preferences();
            }
            Message::Wallpapers(show) => {
                self.preferences.wallpapers = show;
                return Task::batch([self.save_preferences(), self.load_visible_media()]);
            }
            Message::ResetPreferences => {
                self.preferences = Preferences::default();
                self.low_data = !self.preferences.load_photos;
                self.accent_text = preferences::hex(self.preferences.accent);
                return Task::batch([
                    self.save_preferences(),
                    self.update_brand(),
                    self.load_visible_media(),
                ]);
            }
            Message::PreferencesSaved(revision, result) => {
                if revision == self.preference_revision {
                    self.preference_error = result.err();
                }
            }
            Message::SystemAppearance(dark) => {
                if self.system_dark != dark {
                    self.system_dark = dark;
                    self.rebuild_theme();
                    self.preference_revision += 1;
                }
            }
            Message::DetectAppearance => {
                if self.preferences.appearance == Appearance::System {
                    return Task::perform(
                        async {
                            tokio::task::spawn_blocking(preferences::system_is_dark)
                                .await
                                .unwrap_or(false)
                        },
                        Message::SystemAppearance,
                    );
                }
            }
            Message::BrandReady(revision, result) => {
                if revision == self.brand_revision {
                    match result {
                        Ok((logo, header)) => {
                            self.logo = logo;
                            self.header = header;
                        }
                        Err(error) => self.preference_error = Some(error),
                    }
                }
            }
            Message::Navigate(route) => {
                if self.publishing || self.signing_in {
                    return Task::none();
                }
                if self.route != route {
                    self.history.push(self.route.clone());
                }
                self.local_page = None;
                if self.draft_text().is_empty() && self.uploads.is_empty() {
                    self.composition = Composition::Post;
                }
                return self.load(route, 1);
            }
            Message::Back => {
                if self.local_page.take().is_some() {
                    return Task::none();
                }
                if self.publishing || self.signing_in {
                    return Task::none();
                }
                if let Some(route) = self.history.pop() {
                    return self.load(route, 1);
                }
            }
            Message::Refresh => {
                if !self.publishing && !self.signing_in {
                    return self.load(self.route.clone(), self.page_number);
                }
            }
            Message::Older => {
                if !self.loading && !self.publishing && self.page.as_ref().is_some_and(|p| p.older)
                {
                    return self.load(self.route.clone(), self.page_number + 1);
                }
            }
            Message::Newer => {
                if !self.loading && !self.publishing && self.page_number > 1 {
                    return self.load(self.route.clone(), self.page_number - 1);
                }
            }
            Message::SearchChanged(value) => self.search = value,
            Message::Search => {
                let query = self.search.trim().to_string();
                if !query.is_empty() {
                    return self.update(Message::Navigate(Route::Search(query)));
                }
            }
            Message::SessionRestored(client) => {
                if self.generation != 0 {
                    return Task::none();
                }
                self.notice = client.take_session_error();
                self.client = client;
                return self.load(Route::Public, 1);
            }
            Message::Loaded { id, page, result } => {
                if id != self.generation {
                    return Task::none();
                }
                self.loading = false;
                match result {
                    Ok(fetched) => {
                        self.snapshot = false;
                        if self.route == Route::Public
                            && fetched.login_form.is_some()
                            && fetched.composer.is_none()
                            && self.client.saved_account().is_some()
                        {
                            match self.client.forget_session() {
                                Ok(()) => {
                                    self.client = Client::new().expect("TLS client initialization")
                                }
                                Err(error) => self.notice = Some(error),
                            }
                            self.account = None;
                            self.notice.get_or_insert_with(|| {
                                "Your saved session expired. Sign in again.".into()
                            });
                        }
                        if fetched.login_form.is_some()
                            && fetched.account.is_none()
                            && self.account.is_some()
                        {
                            self.account = None;
                            self.notice=Some("Your session expired. Sign in again to continue; your draft is still here.".into());
                        }
                        if fetched.composer.is_some() && fetched.login_form.is_none() {
                            self.account = fetched
                                .account
                                .clone()
                                .or_else(|| self.client.saved_account())
                                .or_else(|| self.account.clone());
                        }
                        if let Some(error) = self.client.take_session_error() {
                            self.notice = Some(error);
                        }
                        self.sync_account_page(&fetched);
                        self.page_number = page;
                        self.page = Some(fetched);
                        return Task::batch([
                            self.load_visible_media(),
                            scrollable::snap_to(
                                scrollable::Id::new("feed"),
                                scrollable::RelativeOffset::START,
                            ),
                        ]);
                    }

                    Err(error) => {
                        if self.page.is_none() && self.route == Route::Public && page == 1 {
                            self.page = ripple_core::parse_page(
                                include_str!("../../../assets/public-preview.html"),
                                &Url::parse(ripple_core::ORIGIN).unwrap(),
                            )
                            .ok();
                            self.snapshot = self.page.is_some();
                        }
                        if self.page.is_none()
                            && matches!(&self.route, Route::Profile(name) if name.trim_start_matches('@').eq_ignore_ascii_case("vgs499p"))
                        {
                            self.page = ripple_core::parse_page(
                                include_str!("../../../assets/public-profile-preview.html"),
                                &self.route.url(1).unwrap(),
                            )
                            .ok();
                            self.snapshot = self.page.is_some();
                        }
                        self.error = Some(error);
                        return self.load_visible_media();
                    }
                }
            }
            Message::ImageLoaded(url, result) => {
                if self.images.contains_key(&url) {
                    self.media_revision += 1;
                    self.image_order.push_back(url.clone());
                    self.images.insert(
                        url,
                        match result {
                            Ok(handle) => Media::Ready(handle),
                            Err(_) => Media::Failed,
                        },
                    );
                    self.trim_images();
                }
            }
            Message::LoadImage(url) => return self.media_task(url),
            Message::LowData(enabled) => {
                self.low_data = enabled;
                self.preferences.load_photos = !enabled;
                self.media_revision += 1;
                let save = self.save_preferences();
                if enabled {
                    self.images.retain(|url, _| url.contains("/pfp/"));
                } else {
                    return Task::batch([save, self.load_visible_media()]);
                }
                return save;
            }
            Message::Open(address) => {
                if let Ok(url) = Url::parse(&address) {
                    if ripple_core::trusted_url(&url) {
                        if let Some(id) = url
                            .path()
                            .strip_prefix("/status/")
                            .and_then(|s| s.parse().ok())
                        {
                            return self.update(Message::Navigate(Route::Thread(id)));
                        }
                        if let Some(name) = url.path().strip_prefix("/~") {
                            return self.update(Message::Navigate(Route::Profile(name.into())));
                        }
                        if url.path() == "/results" {
                            if let Some((_, query)) = url.query_pairs().find(|(key, _)| key == "q")
                            {
                                return self
                                    .update(Message::Navigate(Route::Search(query.into_owned())));
                            }
                        }
                    }
                    if matches!(url.scheme(), "http" | "https")
                        && url.username().is_empty()
                        && url.password().is_none()
                    {
                        return Task::perform(
                            async move { open_browser(url.as_str()).await },
                            Message::BrowserOpened,
                        );
                    }
                }
            }
            Message::ShowLogin => {
                self.local_page = None;
                self.login_open = true;
                self.error = None;
                self.notice = None;
                return text_input::focus("username");
            }
            Message::CloseLogin => {
                if self.local_page.take().is_some() {
                    return Task::none();
                }
                if !self.signing_in {
                    self.login_open = false;
                    self.password.clear();
                }
            }
            Message::Username(value) => self.username = value,
            Message::Password(value) => self.password = value,
            Message::Login => {
                if self.signing_in || self.username.trim().is_empty() || self.password.is_empty() {
                    return Task::none();
                }
                self.generation += 1;
                let id = self.generation;
                self.loading = false;
                self.signing_in = true;
                self.error = None;
                let username = self.username.trim().trim_start_matches('@').to_string();
                let password = std::mem::take(&mut self.password);
                let client = Client::new().expect("TLS client initialization");
                return Task::perform(
                    async move {
                        let result = client
                            .login(username, password)
                            .await
                            .map_err(|e| e.to_string());
                        (client, result)
                    },
                    move |(client, result)| Message::LoggedIn { id, client, result },
                );
            }
            Message::LoggedIn { id, client, result } => {
                if id != self.generation {
                    return Task::none();
                }
                self.signing_in = false;
                match result {
                    Ok(page) => {
                        self.sync_account_page(&page);
                        self.client = client;
                        self.account = Some(page.account.clone().unwrap_or_else(|| {
                            format!("@{}", self.username.trim().trim_start_matches('@'))
                        }));
                        self.page = Some(page);
                        self.page_number = 1;
                        self.route = Route::Public;
                        self.directory = false;
                        self.trend_view = false;
                        self.login_open = false;
                        self.notice = Some(
                            match crate::secure_session::remember(
                                &self.client,
                                self.account.clone().unwrap(),
                            ) {
                                Ok(()) => "Signed in. Your session is saved in Keychain.".into(),
                                Err(error) => error,
                            },
                        );
                        return self.load_visible_media();
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Message::SignOut => {
                if self.publishing {
                    return Task::none();
                }
                if let Err(error) = self.client.forget_session() {
                    self.error = Some(error);
                    return Task::none();
                }
                self.signing_in = false;
                self.client = Client::new().expect("TLS client initialization");
                self.navigation.clear();
                self.form_drafts.clear();
                self.account = None;
                self.password.clear();
                self.images.clear();
                self.history.clear();
                self.draft = text_editor::Content::new();
                self.uploads.clear();
                self.composition = Composition::Post;
                self.notice = Some("Signed out. Saved sign-in removed from Keychain.".into());
                return self.load(Route::Public, 1);
            }
            Message::Edit(action) => {
                if !self.publishing {
                    self.draft.perform(action);
                }
            }
            Message::DiscardDraft => {
                if !self.publishing {
                    self.draft = text_editor::Content::new();
                    self.uploads.clear();
                    self.composition = Composition::Post;
                }
            }
            Message::Publish => {
                if self.publishing || self.account.is_none() || !self.valid_draft() {
                    return Task::none();
                }
                self.generation += 1;
                let id = self.generation;
                self.publishing = true;
                self.loading = false;
                self.error = None;
                self.notice = None;
                let contents = self.draft_text();
                let client = self.client.clone();
                let composition = self.composition.clone();
                let uploads = self.uploads.clone();
                let author = self.account.clone();
                return Task::perform(
                    async move {
                        client
                            .publish_composition(contents, composition, uploads, author)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    move |result| Message::Published { id, result },
                );
            }
            Message::Published { id, result } => {
                if id != self.generation {
                    return Task::none();
                }
                self.publishing = false;
                match result {
                    Ok(page) => {
                        let completed = self.composition.clone();
                        self.page = Some(page);
                        self.page_number = 1;
                        self.route = match &completed {
                            Composition::Reply(id) => Route::Thread(*id),
                            _ => Route::Public,
                        };
                        self.draft = text_editor::Content::new();
                        self.notice = Some(match &completed {
                            Composition::Reply(_) => "Your reply is on Ripple.",
                            Composition::Quote(_) => "Your quote is on Ripple.",
                            Composition::Post => "Your post is on Ripple.",
                        }.into());
                        self.uploads.clear();
                        self.composition = Composition::Post;
                        return Task::batch([
                            self.load_visible_media(),
                            scrollable::snap_to(
                                scrollable::Id::new("feed"),
                                scrollable::RelativeOffset::START,
                            ),
                        ]);
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Message::Dismiss => {
                self.error = None;
                self.notice = None;
            }
            Message::Resize(size) => self.width = size.width,
            Message::FocusSearch => return text_input::focus("search"),
            Message::Quit => return iced::exit(),
            Message::Portal(kind) => {
                if self.account.is_none() {
                    return self.update(Message::ShowLogin);
                }
                let link = self
                    .navigation
                    .iter()
                    .find(|l| {
                        l.label.to_lowercase().contains(&kind)
                            || l.url.to_lowercase().contains(&kind)
                            || (kind == "setting"
                                && ["account", "edit profile", "edit_profile", "editprofile"]
                                    .iter()
                                    .any(|word| {
                                        l.label.to_lowercase().contains(word)
                                            || l.url.to_lowercase().contains(word)
                                    }))
                    })
                    .cloned();
                if let Some(link) = link {
                    return self.update(Message::Navigate(Route::from_navigation(&link.url)));
                }
                self.error = Some(
                    "Your Ripple session has not exposed that page yet. Refresh after signing in."
                        .into(),
                );
            }
            Message::FieldText(form, index, value) => {
                if !self.publishing {
                    if let Some(field) = self
                        .form_drafts
                        .get_mut(form)
                        .and_then(|f| f.get_mut(index))
                    {
                        field.value = value;
                    }
                }
            }
            Message::FieldCheck(form, index, checked) => {
                if !self.publishing {
                    if let Some(field) = self
                        .form_drafts
                        .get_mut(form)
                        .and_then(|f| f.get_mut(index))
                    {
                        field.checked = checked;
                    }
                }
            }
            Message::FieldEdit(form, index, action) => {
                if !self.publishing {
                    if let Some(field) = self
                        .form_drafts
                        .get_mut(form)
                        .and_then(|f| f.get_mut(index))
                    {
                        field.editor.perform(action);
                        field.value = field.editor.text().trim_end_matches('\n').to_string();
                    }
                }
            }
            Message::FormPhoto(form, index) => {
                if !self.publishing {
                    return Task::perform(pick_photos(), move |files| {
                        Message::FormPhotos(form, index, files)
                    });
                }
            }
            Message::FormPhotos(form, index, result) => match result {
                Ok(files) => {
                    if let Some(field) = self
                        .form_drafts
                        .get_mut(form)
                        .and_then(|f| f.get_mut(index))
                    {
                        field.uploads = files;
                    }
                }
                Err(error) => self.error = Some(error),
            },
            Message::SaveForm(index) => {
                if self.account.is_none() || self.publishing || self.snapshot {
                    return Task::none();
                }
                if let Some((form, drafts)) = self
                    .page
                    .as_ref()
                    .and_then(|p| p.editable_forms.get(index))
                    .zip(self.form_drafts.get(index))
                {
                    let mut values = Vec::new();
                    let mut files = Vec::new();
                    for (field, draft) in form.fields.iter().zip(drafts) {
                        if field.kind == FieldKind::File {
                            files.extend(
                                draft
                                    .uploads
                                    .iter()
                                    .cloned()
                                    .map(|u| (field.name.clone(), u)),
                            );
                        } else if field.kind == FieldKind::Checkbox {
                            if draft.checked {
                                values.push((field.name.clone(), field.value.clone()));
                            }
                        } else if field.kind != FieldKind::Password || !draft.value.is_empty() {
                            values.push((field.name.clone(), draft.value.clone()));
                        }
                    }
                    let action = form.form.action.clone();
                    let route = self.route.clone();
                    let client = self.client.clone();
                    self.generation += 1;
                    let id = self.generation;
                    self.publishing = true;
                    self.error = None;
                    self.notice = None;
                    return Task::perform(
                        async move {
                            client
                                .save_settings(route, action, values, files)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        move |result| Message::SettingsSaved { id, result },
                    );
                }
            }
            Message::SettingsSaved { id, result } => {
                if id != self.generation {
                    return Task::none();
                }
                self.publishing = false;
                match result {
                    Ok((page, confirmed)) => {
                        self.sync_account_page(&page);
                        self.page = Some(page);
                        self.notice = Some(
                            if confirmed {
                                "Changes saved."
                            } else {
                                "Changes submitted. Review the refreshed values below."
                            }
                            .into(),
                        );
                        return self.load_visible_media();
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Message::People => {
                return self.update(Message::Navigate(Route::Users));
            }
            Message::DirectoryPage(page) => {
                if !self.loading && !self.publishing && self.route == Route::Users && page > 0 {
                    return self.load(Route::Users, page);
                }
            }
            Message::Trends => {
                if !self.publishing && !self.signing_in {
                    self.local_page = None;
                    let task = self.load(Route::Public, 1);
                    self.trend_view = true;
                    return task;
                }
            }
            Message::BrowserOpened(Err(error)) => self.error = Some(error),
            Message::BrowserOpened(Ok(())) => (),
            Message::ProfileTab(tab) => {
                self.profile_tab = tab;
            }
            Message::Social(address) => {
                if self.account.is_none() {
                    return self.update(Message::ShowLogin);
                }
                if self.publishing || self.signing_in || self.snapshot {
                    return Task::none();
                }
                let discovered = self.page.as_ref().is_some_and(|p| {
                    p.posts.iter().any(|post| {
                        post.like_action.as_deref() == Some(&address)
                            || post.repost_action.as_deref() == Some(&address)
                    }) || p
                        .profile
                        .as_ref()
                        .is_some_and(|profile| profile.follow_action.as_deref() == Some(&address))
                });
                if !discovered {
                    return Task::none();
                }
                self.generation += 1;
                let id = self.generation;
                self.publishing = true;
                let client = self.client.clone();
                let route = self.route.clone();
                let page_number = self.page_number;
                return Task::perform(
                    async move {
                        client
                            .social_action(&address)
                            .await
                            .map_err(|e| e.to_string())?;
                        client.load(&route,page_number).await.map_err(|_|"Action sent, but counts could not be refreshed. Refresh before trying again.".to_string())
                    },
                    move |result| Message::SocialDone { id, result },
                );
            }
            Message::SocialDone { id, result } => {
                if id != self.generation {
                    return Task::none();
                }
                self.publishing = false;
                match result {
                    Ok(fetched) => {
                        if fetched.login_form.is_some() && fetched.account.is_none() {
                            self.account = None;
                            self.error = Some("Your session expired. Sign in again.".into());
                        }
                        if let Some(current) = &mut self.page {
                            for post in &mut current.posts {
                                if let Some(updated) =
                                    fetched.posts.iter().find(|p| p.id == post.id)
                                {
                                    post.likes = updated.likes.clone();
                                    post.reposts = updated.reposts.clone();
                                    post.like_action = updated.like_action.clone();
                                    post.repost_action = updated.repost_action.clone();
                                }
                            }
                            if let Some(profile) = current.profile.as_mut() {
                                if let Some(updated) = fetched
                                    .profile
                                    .as_ref()
                                    .filter(|p| p.handle == profile.handle)
                                {
                                    profile.follow_action = updated.follow_action.clone();
                                    profile.follow_label = updated.follow_label.clone();
                                }
                            }
                        }
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Message::Compose(composition) => {
                if self.account.is_none() {
                    return self.update(Message::ShowLogin);
                }
                if self.publishing || self.signing_in {
                    return Task::none();
                }
                if self.composition != composition
                    && (!self.draft_text().is_empty() || !self.uploads.is_empty())
                {
                    self.error = Some(
                        "Finish or discard the current draft before changing its destination."
                            .into(),
                    );
                    return Task::none();
                }
                let target = match composition {
                    Composition::Reply(id) => Route::Thread(id),
                    _ => Route::Public,
                };
                self.composition = composition;
                return self.load(target, 1);
            }
            Message::PickPhotos => {
                if self.account.is_none() || self.publishing {
                    return Task::none();
                }
                return Task::perform(async { pick_photos().await }, Message::PhotosPicked);
            }
            Message::PhotosPicked(result) => match result {
                Ok(mut files) => {
                    if self.uploads.len() + files.len() > 5
                        || self
                            .uploads
                            .iter()
                            .chain(files.iter())
                            .map(|u| u.bytes.len())
                            .sum::<usize>()
                            > 20 * 1024 * 1024
                    {
                        self.error =
                            Some("Choose at most five images, totaling at most 20 MiB.".into());
                    } else {
                        self.uploads.append(&mut files);
                    }
                }
                Err(error) => self.error = Some(error),
            },
            Message::RemovePhoto(index) => {
                if !self.publishing && index < self.uploads.len() {
                    self.uploads.remove(index);
                }
            }
        }
        Task::none()
    }

    pub fn theme(&self) -> Theme {
        self.cached_theme.clone()
    }
    fn rebuild_theme(&mut self) {
        self.cached_theme = themes::build(&self.preferences, self.system_dark);
    }
    pub fn scale_factor(&self) -> f64 {
        self.preferences.text_size as f64 / 100.0
    }
    fn colors(&self) -> preferences::Colors {
        themes::colors(&self.cached_theme)
    }
    fn save_preferences(&mut self) -> Task<Message> {
        self.rebuild_theme();
        self.preference_revision += 1;
        let revision = self.preference_revision;
        self.preference_error = None;
        let prefs = self.preferences.clone();
        let path = self.settings_path.clone();
        let order = self.save_order.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let mut saved = order
                        .lock()
                        .map_err(|_| "Could not save app preferences.".to_string())?;
                    if revision < *saved {
                        return Ok(());
                    }
                    prefs.save(&path)?;
                    *saved = revision;
                    Ok(())
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            move |result| Message::PreferencesSaved(revision, result),
        )
    }
    fn update_brand(&mut self) -> Task<Message> {
        self.brand_revision += 1;
        let revision = self.brand_revision;
        let accent = self.preferences.accent;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    Ok((
                        preferences::tint(
                            include_bytes!("../../../assets/ui/logo.png"),
                            accent,
                            false,
                        )?,
                        preferences::tint(
                            include_bytes!("../../../assets/ui/header.png"),
                            accent,
                            true,
                        )?,
                    ))
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            move |result| Message::BrandReady(revision, result),
        )
    }

    fn sync_account_page(&mut self, page: &Page) {
        for link in &page.navigation {
            if !self.navigation.iter().any(|l| l.url == link.url) {
                self.navigation.push(link.clone());
            }
        }
        self.form_drafts = page
            .editable_forms
            .iter()
            .map(|form| {
                form.fields
                    .iter()
                    .map(|field| FieldDraft {
                        value: if field.kind == FieldKind::Password {
                            String::new()
                        } else {
                            field.value.clone()
                        },
                        checked: field.checked,
                        editor: text_editor::Content::with_text(&field.value),
                        uploads: Vec::new(),
                    })
                    .collect()
            })
            .collect();
    }

    fn valid_draft(&self) -> bool {
        let contents = self.draft_text();
        (!contents.trim().is_empty() || !self.uploads.is_empty())
            && contents.encode_utf16().count() <= 256
    }

    fn draft_text(&self) -> String {
        self.draft.text().trim_end_matches('\n').to_string()
    }

    fn trim_images(&mut self) {
        let size = |media: &Media| match media {
            Media::Ready(image::Handle::Rgba { width, height, .. }) => {
                *width as usize * *height as usize * 4
            }
            _ => 0,
        };
        let mut resident: usize = self.images.values().map(size).sum();
        let mut checked = 0;
        while resident > 64 * 1024 * 1024 && checked < self.image_order.len() {
            let Some(url) = self.image_order.pop_front() else {
                break;
            };
            let pinned = url.contains("/pfp/")
                || self.page.as_ref().is_some_and(|p| {
                    p.background.as_deref() == Some(&url)
                        || p.profile.as_ref().and_then(|p| p.banner.as_deref()) == Some(&url)
                });
            if pinned {
                self.image_order.push_back(url);
                checked += 1;
                continue;
            }
            if let Some(media) = self.images.remove(&url) {
                resident = resident.saturating_sub(size(&media));
            }
        }
        self.image_order.retain(|url| self.images.contains_key(url));
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let subscriptions = vec![
            #[cfg(target_os = "macos")]
            Subscription::run(crate::mac_menu::events),
            iced::event::listen_with(|event, _, _| {
                if matches!(event, iced::Event::Window(window::Event::Focused)) {
                    Some(Message::DetectAppearance)
                } else {
                    None
                }
            }),
            window::resize_events().map(|(_, size)| Message::Resize(size)),
            keyboard::on_key_press(|key, modifiers| {
                #[cfg(feature = "qa")]
                if key == keyboard::Key::Named(keyboard::key::Named::F6) {
                    return Some(Message::PreviewTheme);
                }
                #[cfg(feature = "qa")]
                if key == keyboard::Key::Named(keyboard::key::Named::F7) {
                    return Some(Message::Local(LocalPage::About));
                }
                #[cfg(feature = "qa")]
                if key == keyboard::Key::Named(keyboard::key::Named::F9) {
                    return Some(Message::BenchStart);
                }
                #[cfg(feature = "qa")]
                if key == keyboard::Key::Named(keyboard::key::Named::F8) {
                    return Some(Message::Capture);
                }
                if modifiers.command() {
                    match key.as_ref() {
                        keyboard::Key::Character(",") => {
                            Some(Message::Local(LocalPage::Appearance))
                        }
                        keyboard::Key::Character("r") => Some(Message::Refresh),
                        keyboard::Key::Character("f") => Some(Message::FocusSearch),
                        keyboard::Key::Character("l") => Some(Message::ShowLogin),
                        keyboard::Key::Character("q") => Some(Message::Quit),
                        _ => None,
                    }
                } else if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                    Some(Message::CloseLogin)
                } else {
                    None
                }
            }),
        ];
        #[cfg(feature = "qa")]
        let subscriptions = {
            let mut subscriptions = subscriptions;
            if self.bench_remaining > 0 {
                subscriptions.push(window::frames().map(Message::BenchFrame));
            }
            subscriptions
        };
        Subscription::batch(subscriptions)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = column![self.topbar(), self.masthead(), self.website_body()]
            .spacing(0)
            .height(Length::Fill);
        container(content)
            .height(Length::Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(theme_colors(theme).background.into()),
                text_color: Some(theme_colors(theme).text),
                ..Default::default()
            })
            .into()
    }

    fn topbar(&self) -> Element<'_, Message> {
        let c = self.colors();
        let mut bar = row![
            button(image(self.logo.clone()).width(82).height(34))
                .padding([4, 10])
                .on_press(Message::Navigate(Route::Public))
                .style(top_button),
            nav_link(
                "Timeline",
                matches!(
                    self.route,
                    Route::Public | Route::Following | Route::Notifications
                ) && !self.directory
                    && !self.trend_view
                    && !self.login_open,
                Message::Navigate(Route::Public)
            ),
            nav_link("Users", self.directory, Message::People),
            nav_link("Trending", self.trend_view, Message::Trends),
            Space::with_width(Length::Fill)
        ]
        .spacing(0)
        .height(46)
        .align_y(alignment::Vertical::Center);
        let search = text_input("Search Ripple", &self.search)
            .id("search")
            .on_input(Message::SearchChanged)
            .on_submit(Message::Search)
            .padding([6, 10])
            .size(13)
            .width(if self.width < 950.0 { 150 } else { 210 })
            .style(move |_, _| text_input::Style {
                background: preferences::mix(c.nav_top, c.nav_text, 0.16).into(),
                border: Border {
                    color: c.nav_bottom,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                icon: c.nav_text,
                placeholder: c.nav_text,
                value: c.nav_text,
                selection: preferences::mix(c.nav_top, c.nav_text, 0.35),
            });
        if self.account.is_some() && self.width > 1050.0 {
            bar = bar
                .push(nav_link(
                    "Notifications",
                    self.route == Route::Notifications,
                    Message::Portal("notif".into()),
                ))
                .push(nav_link(
                    "Settings",
                    matches!(&self.route,Route::Page(p) if p.contains("setting") || p.contains("/account")),
                    Message::Portal("setting".into()),
                ));
        }
        bar = bar.push(nav_link(
            "App",
            self.local_page.is_some(),
            Message::Local(LocalPage::Appearance),
        ));
        bar = bar.push(search).push(
            button(text(self.account.as_deref().unwrap_or("Sign in")).size(13))
                .padding([10, 15])
                .on_press(
                    self.account
                        .as_ref()
                        .map(|a| {
                            Message::Navigate(Route::Profile(a.trim_start_matches('@').into()))
                        })
                        .unwrap_or(Message::ShowLogin),
                )
                .style(top_button),
        );
        container(container(bar).width(Length::Fill).max_width(1030))
            .center_x(Length::Fill)
            .height(46)
            .padding([0, 12])
            .style(move |_| container::Style {
                background: Some(gradient(c.nav_top, c.nav_bottom, std::f32::consts::PI)),
                ..Default::default()
            })
            .into()
    }

    fn masthead(&self) -> Element<'_, Message> {
        let profile = if self.local_page.is_some() {
            None
        } else {
            self.page.as_ref().and_then(|p| p.profile.as_ref())
        };
        let (title, subtitle) = if let Some(page) = self.local_page {
            match page {
                LocalPage::About | LocalPage::License => (
                    "Ripple for Mac".into(),
                    format!(
                        "Version {} • A native Rust client",
                        env!("CARGO_PKG_VERSION")
                    ),
                ),
                _ => (
                    "Make Ripple yours.".into(),
                    "Appearance and preferences for this app.".into(),
                ),
            }
        } else if self.login_open {
            (
                "Welcome back.".to_string(),
                "Sign in to Ripple.".to_string(),
            )
        } else if self.directory {
            (
                "Ripple users".into(),
                "Find familiar faces. Meet someone new.".into(),
            )
        } else if self.trend_view {
            (
                "Trending on Ripple".into(),
                "What everyone's talking about.".into(),
            )
        } else if let Some(profile) = profile {
            (profile.name.clone(), profile.handle.clone())
        } else if let Some(account) = &self.account {
            (
                format!("Hi, {}!", account.trim_start_matches('@')),
                account.clone(),
            )
        } else {
            (
                "Welcome to Ripple.".into(),
                "A little ripple. A whole conversation.".into(),
            )
        };
        let mut hero = row![].spacing(14).align_y(alignment::Vertical::Center);
        if let Some(profile) =
            profile.filter(|_| !self.login_open && !self.directory && !self.trend_view)
        {
            hero = hero.push(self.avatar(profile.avatar.as_deref(), &profile.handle, 84));
        } else if let Some(account) = &self.account {
            let own = self.page.as_ref().and_then(|p| {
                p.posts
                    .iter()
                    .find(|p| p.handle.eq_ignore_ascii_case(account))
            });
            if let Some(own) = own {
                hero = hero.push(self.avatar(own.avatar.as_deref(), account, 84));
            }
        }
        hero = hero
            .push(
                column![
                    text(title).size(37).font(BOLD).color(Color::WHITE),
                    text(subtitle).size(17).color(Color::WHITE)
                ]
                .spacing(4),
            )
            .push(Space::with_width(Length::Fill));
        if let Some(profile) = profile {
            if let Some(action) = &profile.follow_action {
                hero = hero.push(
                    button(
                        text(if profile.follow_label.is_empty() {
                            "Follow"
                        } else {
                            &profile.follow_label
                        })
                        .size(13)
                        .font(BOLD),
                    )
                    .on_press(Message::Social(action.clone()))
                    .padding([8, 15])
                    .style(bootstrap_button),
                );
            }
        }
        if let Some(profile) =
            profile.filter(|_| !self.login_open && !self.directory && !self.trend_view)
        {
            hero = hero.push(
                column(
                    profile
                        .stats
                        .iter()
                        .map(|s| text(s).size(14).font(BOLD).color(Color::WHITE).into()),
                )
                .spacing(7),
            );
        }
        let foreground = container(container(hero).max_width(916).width(Length::Fill))
            .center_x(Length::Fill)
            .center_y(145)
            .height(145)
            .padding([0, 22]);
        let background: Element<'_, Message> = if let Some(Media::Ready(handle)) = profile
            .and_then(|p| p.banner.as_ref())
            .and_then(|url| self.images.get(url))
        {
            image(handle.clone())
                .width(Length::Fill)
                .height(145)
                .content_fit(iced::ContentFit::Cover)
                .into()
        } else {
            image(self.header.clone())
                .width(Length::Fill)
                .height(145)
                .content_fit(iced::ContentFit::Cover)
                .into()
        };
        stack![background, foreground]
            .height(145)
            .width(Length::Fill)
            .into()
    }

    fn website_body(&self) -> Element<'_, Message> {
        if let Some(page) = self.local_page {
            return self.local_view(page);
        }
        let search = text_input("Search posts and #hashtags", &self.search)
            .id("search")
            .on_input(Message::SearchChanged)
            .on_submit(Message::Search)
            .padding([8, 12])
            .size(13)
            .style(|_, _| text_input::Style {
                background: self.colors().surface.into(),
                border: Border {
                    color: self.colors().border,
                    width: 1.0,
                    radius: 16.0.into(),
                },
                icon: self.colors().muted,
                placeholder: self.colors().muted,
                value: self.colors().text,
                selection: self.preferences.accent_color(),
            });
        let mut feed = Column::new().spacing(0).width(Length::Fill);
        feed = feed.push(
            container(
                row![
                    search,
                    button(text("Search").size(12).font(BOLD))
                        .on_press(Message::Search)
                        .padding([7, 12])
                        .style(bootstrap_button)
                ]
                .spacing(9),
            )
            .padding([13, 16])
            .style(|theme: &Theme| container::Style {
                background: Some(theme_colors(theme).inset.into()),
                ..Default::default()
            }),
        );
        if self.snapshot {
            feed = feed.push(
                container(
                    text("Offline preview · Saved public posts from 20:36")
                        .size(11)
                        .color(if self.preferences.dark(self.system_dark) {
                            rgb(232, 204, 143)
                        } else {
                            rgb(130, 98, 32)
                        }),
                )
                .width(Length::Fill)
                .padding([9, 16])
                .style(|theme: &Theme| container::Style {
                    background: Some(
                        if theme.extended_palette().is_dark {
                            rgb(70, 57, 35)
                        } else {
                            rgb(252, 247, 226)
                        }
                        .into(),
                    ),
                    ..Default::default()
                }),
            );
        }
        if let Some(error) = self
            .error
            .as_ref()
            .filter(|e| !self.snapshot || !e.contains("530"))
        {
            let message = if error.contains("530") {
                "Ripple is temporarily offline (HTTP 530). Try Refresh when the site returns."
            } else {
                error
            };
            feed = feed.push(self.banner(message, true));
        }
        if let Some(notice) = &self.notice {
            feed = feed.push(self.banner(notice, false));
        }
        if self.login_open {
            feed = feed.push(self.login_panel());
        } else if matches!(self.route, Route::Page(_)) {
            feed = feed.push(self.account_page());
        } else if self.directory {
            feed = feed.push(section_title("Users"));
            if let Some(page) = &self.page {
                if !page.heading.is_empty() {
                    feed = feed.push(
                        container(text(&page.heading).size(13).color(self.colors().muted))
                            .padding([10, 22]),
                    );
                }
                let per_row = if self.width >= 1050.0 { 4 } else { 3 };
                for group in page.people.chunks(per_row) {
                    let mut cards = Row::new().spacing(12);
                    for person in group {
                        let name = if person.verified {
                            format!("{} ✓", person.name)
                        } else {
                            person.name.clone()
                        };
                        cards = cards.push(
                            button(
                                column![
                                    self.avatar(person.avatar.as_deref(), &person.handle, 112),
                                    text(name).size(14).font(BOLD),
                                    text(&person.handle).size(12).color(self.colors().muted)
                                ]
                                .spacing(8)
                                .align_x(alignment::Horizontal::Center),
                            )
                            .on_press(Message::Navigate(Route::Profile(
                                person.handle.trim_start_matches('@').into(),
                            )))
                            .padding(14)
                            .width(Length::Fill)
                            .style(bootstrap_button),
                        );
                    }
                    for _ in group.len()..per_row {
                        cards = cards.push(Space::with_width(Length::Fill));
                    }
                    feed = feed.push(container(cards).padding([8, 18]));
                }
                feed = feed.push(
                    container(
                        row![
                            button("< Previous")
                                .on_press_maybe(
                                    (!self.loading && self.page_number > 1)
                                        .then(|| Message::DirectoryPage(self.page_number - 1))
                                )
                                .style(bootstrap_button)
                                .padding([8, 14]),
                            Space::with_width(Length::Fill),
                            text(format!("Page {}", self.page_number)).size(13),
                            Space::with_width(Length::Fill),
                            button("Next >")
                                .on_press_maybe(
                                    (!self.loading && page.older)
                                        .then_some(Message::DirectoryPage(self.page_number + 1))
                                )
                                .style(bootstrap_button)
                                .padding([8, 14]),
                        ]
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding(20),
                );
            } else {
                feed = feed.push(
                    container(text(if self.loading {
                        "Loading users..."
                    } else {
                        "Could not load users. Use Refresh to retry."
                    }))
                    .padding(22),
                );
            }
        } else if self.trend_view {
            feed = feed.push(section_title("Trending topics"));
            feed = feed.push(
                container(
                    text(
                        "Topics from Ripple's sidebar. The website's Trending page is coming soon.",
                    )
                    .size(12)
                    .color(self.colors().muted),
                )
                .padding([10, 22]),
            );
            if let Some(page) = &self.page {
                for trend in &page.trends {
                    feed = feed.push(
                        container(
                            button(text(trend).size(21).color(self.colors().link))
                                .padding(0)
                                .on_press(Message::Navigate(Route::Search(trend.clone())))
                                .style(quiet),
                        )
                        .padding([15, 22]),
                    );
                }
            }
        } else {
            let mut tabs = row![].spacing(0).align_y(alignment::Vertical::Bottom);
            if !self.history.is_empty() {
                tabs = tabs.push(
                    button(text("< Back").size(12))
                        .on_press(Message::Back)
                        .padding([11, 14])
                        .style(quiet),
                );
            }
            let tab_name = match &self.route {
                Route::Public => "Public".to_string(),
                Route::Following => "Following".into(),
                Route::Notifications => "Notifications".into(),
                Route::Users => "Users".into(),
                Route::Profile(_) => "Posts".into(),
                Route::Search(q) => format!("Search: {q}"),
                Route::Thread(_) => "Conversation".into(),
                Route::Page(_) => "Your account".into(),
            };
            if matches!(
                self.route,
                Route::Public | Route::Following | Route::Notifications
            ) {
                for (label, route) in [
                    ("Public", Route::Public),
                    ("Following", Route::Following),
                    ("Notifications", Route::Notifications),
                ] {
                    let active = self.route == route;
                    let message = if route == Route::Public {
                        Message::Navigate(route)
                    } else if route == Route::Following {
                        Message::Portal("following".into())
                    } else {
                        Message::Portal("notif".into())
                    };
                    tabs = tabs.push(
                        button(
                            text(label)
                                .size(13)
                                .font(if active { BOLD } else { Font::DEFAULT })
                                .color(if active {
                                    self.colors().link
                                } else {
                                    self.colors().muted
                                }),
                        )
                        .on_press(message)
                        .padding([10, 12])
                        .style(quiet),
                    );
                }
            } else {
                tabs = tabs.push(
                    container(
                        text(if self.route == Route::Public {
                            "Latest posts".to_string()
                        } else {
                            tab_name
                        })
                        .size(15)
                        .font(BOLD)
                        .color(self.colors().text),
                    )
                    .padding([10, 0]),
                );
            }
            if self.page.as_ref().is_some_and(|p| p.profile.is_some()) {
                tabs = tabs
                    .push(
                        button(text("Posts").size(12).color(if self.profile_tab == 0 {
                            self.colors().link
                        } else {
                            self.colors().muted
                        }))
                        .padding([11, 10])
                        .on_press(Message::ProfileTab(0))
                        .style(quiet),
                    )
                    .push(
                        button(text("Followers").size(12).color(if self.profile_tab == 1 {
                            self.colors().link
                        } else {
                            self.colors().muted
                        }))
                        .padding([11, 10])
                        .on_press(Message::ProfileTab(1))
                        .style(quiet),
                    )
                    .push(
                        button(text("Following").size(12).color(if self.profile_tab == 2 {
                            self.colors().link
                        } else {
                            self.colors().muted
                        }))
                        .padding([11, 10])
                        .on_press(Message::ProfileTab(2))
                        .style(quiet),
                    );
            }
            tabs = tabs.push(Space::with_width(Length::Fill)).push(
                button(
                    text(if self.loading {
                        "Loading..."
                    } else {
                        "Refresh"
                    })
                    .size(11),
                )
                .padding([11, 14])
                .on_press_maybe(
                    (!self.loading && !self.signing_in && !self.publishing)
                        .then_some(Message::Refresh),
                )
                .style(quiet),
            );
            feed = feed
                .push(container(tabs).padding([10, 20]))
                .push(divider(1));
            if let Some(page) = &self.page {
                if let Some(profile) = &page.profile {
                    if !profile.bio.is_empty() {
                        feed = feed
                            .push(
                                container(text(&profile.bio).size(13).line_height(1.4))
                                    .padding([16, 22]),
                            )
                            .push(divider(1));
                    }
                }
                if matches!(self.route, Route::Public | Route::Following)
                    || matches!(self.composition, Composition::Reply(_))
                {
                    feed = feed.push(self.composer()).push(divider(1));
                }
                if self.profile_tab > 0 {
                    if let Some(profile) = &page.profile {
                        let people = if self.profile_tab == 1 {
                            &profile.followers
                        } else {
                            &profile.following
                        };
                        for person in people {
                            feed = feed.push(self.person_row(person, true)).push(divider(1));
                        }
                        if people.is_empty() {
                            feed = feed.push(
                                container(
                                    text("No users listed.").size(13).color(self.colors().muted),
                                )
                                .padding(22),
                            );
                        }
                    }
                } else {
                    if self.route == Route::Notifications {
                        for item in &page.notifications {
                            let mut entry =
                                column![text(&item.text).size(14).line_height(1.4)].spacing(8);
                            for link in &item.links {
                                entry = entry.push(
                                    button(text(&link.label).size(12).color(self.colors().link))
                                        .on_press(Message::Open(link.url.clone()))
                                        .style(quiet)
                                        .padding(0),
                                );
                            }
                            feed = feed.push(container(entry).padding(20)).push(divider(1));
                        }
                    }
                    if page.posts.is_empty()
                        && (self.route != Route::Notifications || page.notifications.is_empty())
                    {
                        feed = feed.push(
                            container(
                                column![
                                    text(if self.route == Route::Notifications {
                                        "No notifications found."
                                    } else {
                                        "No posts found."
                                    })
                                    .size(17)
                                    .font(BOLD),
                                    text(if self.route == Route::Notifications {
                                        "Replies and mentions will appear here."
                                    } else {
                                        "Try another search or refresh."
                                    })
                                    .size(13)
                                    .color(self.colors().muted)
                                ]
                                .spacing(8),
                            )
                            .padding(24),
                        );
                    }
                    for post in &page.posts {
                        feed = feed
                            .push(iced::widget::lazy(
                                (
                                    post.id,
                                    self.generation,
                                    self.media_revision,
                                    self.low_data,
                                    self.preference_revision,
                                ),
                                move |_| self.post_card(post),
                            ))
                            .push(divider(1));
                    }
                    if self.snapshot {
                        feed = feed.push(
                            container(
                                text("End of saved preview. Refresh to reconnect.")
                                    .size(12)
                                    .color(self.colors().muted),
                            )
                            .padding(20),
                        );
                    } else {
                        feed = feed.push(
                            container(
                                row![
                                    button("< Newer")
                                        .on_press_maybe(
                                            (!self.loading
                                                && !self.publishing
                                                && self.page_number > 1)
                                                .then_some(Message::Newer)
                                        )
                                        .padding([8, 15])
                                        .style(bootstrap_button),
                                    Space::with_width(Length::Fill),
                                    text(format!("Page {}", self.page_number)).size(13),
                                    Space::with_width(Length::Fill),
                                    button("Older >")
                                        .on_press_maybe(
                                            (!self.loading && !self.publishing && page.older)
                                                .then_some(Message::Older)
                                        )
                                        .padding([8, 15])
                                        .style(bootstrap_button)
                                ]
                                .align_y(alignment::Vertical::Center),
                            )
                            .padding(20),
                        );
                    }
                }
            } else if self.loading {
                feed = feed.push(
                    container(
                        text("Loading Ripple...")
                            .size(14)
                            .color(self.colors().muted),
                    )
                    .padding(24),
                );
            } else {
                feed = feed.push(
                    container(
                        button("Try again")
                            .on_press(Message::Refresh)
                            .style(bootstrap_button)
                            .padding(12),
                    )
                    .padding(24),
                );
            }
        }
        let mut columns = row![container(feed).width(Length::Fill).style(white_panel)]
            .spacing(18)
            .align_y(alignment::Vertical::Top);
        if self.width >= 850.0 && !self.login_open && !self.directory {
            columns = columns.push(self.right_panel());
        }
        let content = container(columns).width(Length::Fill).max_width(1030);
        let scrolling = scrollable(container(content).center_x(Length::Fill).padding([20, 24]))
            .id(scrollable::Id::new("feed"))
            .height(Length::Fill);
        if let Some(Media::Ready(handle)) = self
            .page
            .as_ref()
            .filter(|_| self.preferences.wallpapers)
            .and_then(|p| p.background.as_ref())
            .and_then(|url| self.images.get(url))
        {
            stack![
                image(handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Cover),
                scrolling
            ]
            .height(Length::Fill)
            .into()
        } else {
            container(scrolling)
                .style(|_| container::Style {
                    background: Some(self.colors().background.into()),
                    ..Default::default()
                })
                .into()
        }
    }

    fn avatar(&self, url: Option<&str>, fallback: &str, size: u16) -> Element<'static, Message> {
        let interior: Element<'static, Message> =
            if let Some(Media::Ready(handle)) = url.and_then(|u| self.images.get(u)) {
                image(handle.clone())
                    .width(size)
                    .height(size)
                    .content_fit(iced::ContentFit::Cover)
                    .into()
            } else {
                let colors = [
                    rgb(102, 132, 164),
                    rgb(137, 116, 158),
                    rgb(91, 143, 137),
                    rgb(169, 128, 94),
                    rgb(112, 132, 148),
                ];
                let color = colors[fallback.bytes().map(usize::from).sum::<usize>() % colors.len()];
                let top = Color {
                    r: (color.r + 0.18).min(1.0),
                    g: (color.g + 0.18).min(1.0),
                    b: (color.b + 0.18).min(1.0),
                    a: 1.0,
                };
                container(
                    text(
                        fallback
                            .trim_start_matches('@')
                            .chars()
                            .next()
                            .unwrap_or('r')
                            .to_uppercase()
                            .to_string(),
                    )
                    .size(size as f32 * 0.5)
                    .font(BOLD)
                    .color(Color::WHITE),
                )
                .width(size)
                .height(size)
                .center_x(size)
                .center_y(size)
                .style(move |_| container::Style {
                    background: Some(gradient(top, color, std::f32::consts::PI)),
                    border: border::rounded(5),
                    ..Default::default()
                })
                .into()
            };
        container(interior)
            .padding(2)
            .style(|theme: &Theme| container::Style {
                background: Some(theme_colors(theme).surface.into()),
                border: Border {
                    color: theme_colors(theme).border,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn post_card(&self, post: &Post) -> Element<'static, Message> {
        let mut author = row![text(post.author.clone())
            .size(14)
            .font(BOLD)
            .color(self.colors().text)]
        .spacing(4)
        .align_y(alignment::Vertical::Center);
        if post.verified {
            author = author.push(text("verified").size(9).color(self.colors().text));
        }
        author = author.push(
            text(post.handle.clone())
                .size(12)
                .color(self.colors().muted),
        );
        let header = row![
            button(author)
                .padding(0)
                .on_press(Message::Navigate(Route::Profile(
                    post.handle.trim_start_matches('@').into()
                )))
                .style(quiet),
            Space::with_width(Length::Fill),
            text(post.time.clone()).size(11).color(self.colors().muted)
        ]
        .spacing(5)
        .align_y(alignment::Vertical::Center);
        let mut body = column![header].spacing(4);
        if let Some(reply) = &post.reply_to {
            body = body.push(
                button(text(reply.label.clone()).size(12).color(self.colors().link))
                    .padding(0)
                    .on_press(Message::Open(reply.url.clone()))
                    .style(quiet),
            );
        }
        if !post.text.is_empty() {
            body = body.push(
                text(post.text.clone())
                    .size(if self.preferences.density == Density::Compact {
                        13
                    } else {
                        15
                    })
                    .line_height(1.4)
                    .shaping(iced::widget::text::Shaping::Advanced),
            );
        }
        for url in &post.images {
            let media: Element<'static, Message> = match self.images.get(url) {
                Some(Media::Ready(handle)) => button(
                    image(handle.clone())
                        .width(Length::Fill)
                        .height(380)
                        .content_fit(iced::ContentFit::Contain),
                )
                .padding(0)
                .width(Length::Fill)
                .on_press(Message::Open(url.clone()))
                .style(quiet)
                .into(),
                state => {
                    let label = match state {
                        Some(Media::Loading) => "Loading photo…",
                        Some(Media::Failed) => "Photo unavailable while Ripple is offline",
                        _ => "Load photo",
                    };
                    button(
                        container(text(label).size(12).color(self.colors().muted))
                            .center_x(Length::Fill)
                            .height(57)
                            .center_y(57)
                            .style(soft_card),
                    )
                    .padding(0)
                    .width(Length::Fill)
                    .on_press_maybe(
                        (!matches!(state, Some(Media::Loading)))
                            .then_some(Message::LoadImage(url.clone())),
                    )
                    .style(quiet)
                    .into()
                }
            };
            body = body.push(media);
        }
        for link in post.links.iter().take(4) {
            let label = if link.label.is_empty() {
                link.url.clone()
            } else {
                link.label.clone()
            };
            let short = if label.chars().count() > 70 {
                format!("{}...", label.chars().take(67).collect::<String>())
            } else {
                label
            };
            body = body.push(
                button(text(short).size(12).color(self.colors().link))
                    .padding(0)
                    .on_press(Message::Open(link.url.clone()))
                    .style(quiet),
            );
        }
        let metrics = row![
            button(metric(&self.heart, &post.likes))
                .on_press_maybe(
                    post.like_action
                        .as_ref()
                        .map(|a| Message::Social(a.clone()))
                )
                .padding(0)
                .style(quiet),
            button(metric(&self.comment, &post.replies))
                .on_press(Message::Navigate(Route::Thread(post.id)))
                .padding(0)
                .style(quiet),
            button(metric(&self.repost, &post.reposts))
                .on_press_maybe(
                    post.repost_action
                        .as_ref()
                        .map(|a| Message::Social(a.clone()))
                )
                .padding(0)
                .style(quiet),
            button(image(self.quote.clone()).width(13).height(13))
                .on_press(Message::Compose(Composition::Quote(post.id)))
                .padding(0)
                .style(quiet),
            button(text("Reply").size(10).color(self.colors().text))
                .on_press(Message::Compose(Composition::Reply(post.id)))
                .padding(0)
                .style(quiet),
            Space::with_width(Length::Fill),
            button(text("View thread").size(10).color(self.colors().muted))
                .padding(0)
                .on_press(Message::Navigate(Route::Thread(post.id)))
                .style(quiet)
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);
        body = body.push(container(metrics).padding([4, 0]));
        container(
            row![
                button(self.avatar(post.avatar.as_deref(), &post.handle, 46))
                    .padding(0)
                    .on_press(Message::Navigate(Route::Profile(
                        post.handle.trim_start_matches('@').into()
                    )))
                    .style(quiet),
                body.width(Length::Fill)
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Top),
        )
        .padding([
            if self.preferences.density == Density::Compact {
                10
            } else {
                17
            },
            18,
        ])
        .width(Length::Fill)
        .into()
    }

    fn composer(&self) -> Element<'_, Message> {
        if self.account.is_none() {
            return container(
                row![
                    column![
                        text("Join the conversation")
                            .size(14)
                            .font(BOLD)
                            .color(self.colors().text),
                        text("Sign in to post, reply, and follow people.")
                            .size(12)
                            .color(self.colors().muted)
                    ]
                    .spacing(4),
                    Space::with_width(Length::Fill),
                    button(text("Sign in").size(12).font(BOLD))
                        .on_press(Message::ShowLogin)
                        .padding([7, 16])
                        .style(primary)
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center),
            )
            .padding([17, 18])
            .width(Length::Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(theme_colors(theme).inset.into()),
                ..Default::default()
            })
            .into();
        }
        let remaining = 256_i64 - self.draft_text().encode_utf16().count() as i64;
        let label = match self.composition {
            Composition::Post => "New public post".to_string(),
            Composition::Reply(id) => format!("Reply to post #{id}"),
            Composition::Quote(id) => format!("Quote post #{id}"),
        };
        let mut composer = column![
            row![
                text(label).size(12).font(BOLD).color(self.colors().text),
                Space::with_width(Length::Fill),
                button(text("Discard draft").size(11))
                    .on_press_maybe((!self.publishing).then_some(Message::DiscardDraft))
                    .padding(0)
                    .style(quiet)
            ]
            .align_y(alignment::Vertical::Center),
            text_editor(&self.draft)
                .placeholder("What's on your mind?")
                .on_action(Message::Edit)
                .height(65)
                .size(14)
                .padding(10),
            row![
                text(remaining).size(18).font(BOLD).color(if remaining < 0 {
                    self.colors().danger
                } else {
                    self.colors().muted
                }),
                button(text("Attach photos").size(11))
                    .on_press_maybe((!self.publishing).then_some(Message::PickPhotos))
                    .padding([5, 9])
                    .style(bootstrap_button),
                Space::with_width(Length::Fill),
                button(
                    text(if self.publishing {
                        "Publishing..."
                    } else {
                        "Post"
                    })
                    .size(13)
                )
                .padding([7, 16])
                .on_press_maybe(
                    (!self.publishing && self.valid_draft()).then_some(Message::Publish)
                )
                .style(primary)
            ]
            .spacing(14)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(11);
        for (index, file) in self.uploads.iter().enumerate() {
            composer = composer.push(
                row![
                    text(format!(
                        "{} ({:.0} KB)",
                        file.name,
                        file.bytes.len() as f32 / 1024.0
                    ))
                    .size(11)
                    .color(self.colors().muted),
                    button(text("Remove").size(10))
                        .on_press_maybe((!self.publishing).then_some(Message::RemovePhoto(index)))
                        .padding(0)
                        .style(quiet)
                ]
                .spacing(10),
            );
        }
        container(composer)
            .padding([18, 21])
            .width(Length::Fill)
            .into()
    }

    fn login_panel(&self) -> Element<'_, Message> {
        let mut fields = column![
            text("Sign in to Ripple").size(25).font(BOLD),
            Space::with_height(5), text("Username").size(13).font(BOLD),
            text_input("Your username", &self.username).id("username").on_input(Message::Username).padding(10).size(14),
            text("Password").size(13).font(BOLD),
            text_input("Your password", &self.password).secure(true).on_input(Message::Password).on_submit(Message::Login).padding(10).size(14),
            button(text(if self.signing_in { "Signing in..." } else { "Sign in" }).size(14)).padding([8,18])
                .on_press_maybe((!self.signing_in && !self.username.trim().is_empty() && !self.password.is_empty()).then_some(Message::Login)).style(primary),
            text("Stay signed in across restarts. Your session is saved in Keychain; your password is never stored.").size(12).color(self.colors().muted).line_height(1.5),
        ].spacing(12).max_width(360);
        if !self.signing_in {
            fields = fields.push(
                button(
                    text("< Continue browsing")
                        .size(12)
                        .color(self.colors().link),
                )
                .on_press(Message::CloseLogin)
                .style(quiet),
            );
        }
        container(container(fields).center_x(Length::Fill))
            .padding([35, 24])
            .width(Length::Fill)
            .into()
    }

    fn banner<'a>(&self, message: &'a str, error: bool) -> Element<'a, Message> {
        container(
            row![
                text(message).size(12).line_height(1.35).width(Length::Fill),
                button(text("x").size(13))
                    .on_press(Message::Dismiss)
                    .padding(3)
                    .style(quiet)
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .padding([10, 16])
        .width(Length::Fill)
        .style(move |theme| {
            let c = theme_colors(theme);
            let accent = if error { c.danger } else { c.success };
            container::Style {
                background: Some(preferences::mix(accent, c.surface, 0.88).into()),
                text_color: Some(c.text),
                ..Default::default()
            }
        })
        .into()
    }

    fn person_row<'a>(
        &'a self,
        person: &'a ripple_core::Person,
        directory: bool,
    ) -> Element<'a, Message> {
        let mut details = column![row![
            text(&person.name).size(12).font(BOLD),
            text(&person.handle).size(11).color(self.colors().muted)
        ]
        .spacing(4),]
        .spacing(5);
        let action = if directory { "View profile" } else { "Profile" };
        details = details.push(
            row![
                button(text(action).size(12))
                    .on_press(Message::Navigate(Route::Profile(
                        person.handle.trim_start_matches('@').into()
                    )))
                    .style(bootstrap_button)
                    .padding([4, 11]),
                container(text(&person.followers).size(11).color(self.colors().muted))
                    .padding([4, 7])
                    .style(soft_card)
            ]
            .spacing(8),
        );
        container(
            row![
                self.avatar(person.avatar.as_deref(), &person.handle, 44),
                details
            ]
            .spacing(9),
        )
        .padding(if directory { [15, 21] } else { [12, 0] })
        .width(Length::Fill)
        .into()
    }

    fn account_page(&self) -> Element<'_, Message> {
        let mut content = Column::new().spacing(18).padding(22).width(Length::Fill);
        if let Some(page) = &self.page {
            let is_notifications =
                matches!(&self.route,Route::Page(url) if url.to_lowercase().contains("notif"));
            content = content.push(
                text(if is_notifications {
                    "Notifications"
                } else {
                    "Account and profile settings"
                })
                .size(23)
                .font(BOLD),
            );
            if is_notifications {
                for item in &page.notifications {
                    let mut entry = column![text(&item.text)
                        .size(14)
                        .line_height(1.4)
                        .shaping(iced::widget::text::Shaping::Advanced)]
                    .spacing(8);
                    for link in &item.links {
                        entry = entry.push(
                            button(text(&link.label).size(12).color(self.colors().link))
                                .on_press(Message::Open(link.url.clone()))
                                .padding(0)
                                .style(quiet),
                        );
                    }
                    content = content.push(entry).push(divider(1));
                }
                if page.notifications.is_empty() {
                    content = content.push(
                        text(if page.document_text.is_empty() {
                            "No notifications on this page."
                        } else {
                            &page.document_text
                        })
                        .size(14)
                        .line_height(1.45),
                    );
                }
            } else {
                for (form_index, form) in page.editable_forms.iter().enumerate() {
                    let Some(drafts) = self.form_drafts.get(form_index) else {
                        continue;
                    };
                    let mut fields = column![text(&form.title).size(17).font(BOLD)].spacing(12);
                    for (index, (field, draft)) in form.fields.iter().zip(drafts).enumerate() {
                        if field.kind != FieldKind::Checkbox {
                            fields = fields.push(text(&field.label).size(12).font(BOLD));
                        }
                        let editor: Element<'_, Message> = match field.kind {
                            FieldKind::Text | FieldKind::Password => {
                                text_input(&field.label, &draft.value)
                                    .secure(field.kind == FieldKind::Password)
                                    .on_input(move |v| Message::FieldText(form_index, index, v))
                                    .padding(10)
                                    .size(14)
                                    .into()
                            }
                            FieldKind::Multiline => text_editor(&draft.editor)
                                .on_action(move |a| Message::FieldEdit(form_index, index, a))
                                .height(100)
                                .padding(10)
                                .size(14)
                                .into(),
                            FieldKind::Checkbox => checkbox(&field.label, draft.checked)
                                .on_toggle(move |v| Message::FieldCheck(form_index, index, v))
                                .size(18)
                                .text_size(13)
                                .into(),
                            FieldKind::Select => pick_list(
                                field.choices.clone(),
                                field
                                    .choices
                                    .iter()
                                    .find(|c| c.value == draft.value)
                                    .cloned(),
                                move |c: ripple_core::Choice| {
                                    Message::FieldText(form_index, index, c.value)
                                },
                            )
                            .padding(9)
                            .text_size(13)
                            .into(),
                            FieldKind::File => column![
                                button(text("Choose image...").size(12))
                                    .on_press(Message::FormPhoto(form_index, index))
                                    .padding([7, 12])
                                    .style(bootstrap_button),
                                text(
                                    draft
                                        .uploads
                                        .iter()
                                        .map(|u| u.name.clone())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )
                                .size(11)
                                .color(self.colors().muted)
                            ]
                            .spacing(6)
                            .into(),
                        };
                        fields = fields.push(editor);
                    }
                    fields = fields.push(
                        button(
                            text(if self.publishing {
                                "Saving..."
                            } else {
                                "Save changes"
                            })
                            .size(13)
                            .font(BOLD),
                        )
                        .on_press_maybe((!self.publishing).then_some(Message::SaveForm(form_index)))
                        .padding([9, 18])
                        .style(primary),
                    );
                    content = content.push(fields).push(divider(1));
                }
                if page.editable_forms.is_empty() {
                    content = content.push(
                        text("This page does not expose a supported editable form.")
                            .size(13)
                            .color(self.colors().muted),
                    );
                }
            }
            for link in &page.navigation {
                content = content.push(
                    button(text(&link.label).size(12).color(self.colors().link))
                        .on_press(Message::Navigate(Route::from_navigation(&link.url)))
                        .padding(0)
                        .style(quiet),
                );
            }
        } else {
            content = content.push(
                text("Loading your account...")
                    .size(13)
                    .color(self.colors().muted),
            );
        }
        content.into()
    }

    fn right_panel(&self) -> Element<'_, Message> {
        let mut sidebar = Column::new().spacing(0).width(Length::Fill);
        if let Some(page) = &self.page {
            if let Some(profile) = page.profile.as_ref().filter(|p| !p.details.is_empty()) {
                sidebar = sidebar.push(text("About Me").size(18).font(BOLD));
                for detail in &profile.details {
                    sidebar = sidebar
                        .push(container(text(detail).size(13).line_height(1.4)).padding([5, 0]));
                }
                sidebar = sidebar.push(divider(1));
            }
            sidebar = sidebar
                .push(
                    text("Trending topics")
                        .size(17)
                        .font(BOLD)
                        .color(self.colors().text),
                )
                .push(Space::with_height(7));
            let mut tags = Column::new().spacing(4);
            for (index, chunk) in page.trends.chunks(2).enumerate() {
                let mut pair = row![].spacing(11).align_y(alignment::Vertical::Center);
                for trend in chunk {
                    pair = pair.push(
                        button(
                            text(trend)
                                .size(if index == 0 { 15 } else { 13 })
                                .color(self.colors().link),
                        )
                        .padding(0)
                        .on_press(Message::Navigate(Route::Search(trend.clone())))
                        .style(quiet),
                    );
                }
                tags = tags.push(pair);
            }
            sidebar = sidebar
                .push(tags)
                .push(container(divider(1)).padding([20, 0]))
                .push(
                    text("Recently active")
                        .size(17)
                        .font(BOLD)
                        .color(self.colors().text),
                );
            for person in page.people.iter().take(5) {
                sidebar = sidebar
                    .push(self.person_row(person, false))
                    .push(divider(1));
            }
        } else {
            sidebar = sidebar
                .push(
                    text("Trending topics")
                        .size(17)
                        .font(BOLD)
                        .color(self.colors().text),
                )
                .push(
                    text("Waiting for Ripple...")
                        .size(12)
                        .color(self.colors().muted),
                );
        }
        sidebar = sidebar
            .push(Space::with_height(20))
            .push(
                toggler(self.low_data)
                    .label("Low data mode")
                    .text_size(11)
                    .size(16)
                    .spacing(7)
                    .on_toggle(Message::LowData),
            )
            .push(Space::with_height(14))
            .push(
                text(format!("Ripple for Mac · {}", env!("CARGO_PKG_VERSION")))
                    .size(10)
                    .color(self.colors().muted),
            );
        sidebar = sidebar
            .push(
                button(text("Notifications").size(12).color(self.colors().link))
                    .on_press(Message::Portal("notif".into()))
                    .style(quiet)
                    .padding([7, 0]),
            )
            .push(
                button(
                    text("Account and profile settings")
                        .size(12)
                        .color(self.colors().link),
                )
                .on_press(Message::Portal("setting".into()))
                .style(quiet)
                .padding([7, 0]),
            );
        sidebar = sidebar
            .push(
                button(text("App settings").size(12).color(self.colors().link))
                    .on_press(Message::Local(LocalPage::Appearance))
                    .padding([7, 0])
                    .style(quiet),
            )
            .push(
                button(text("About Ripple").size(12).color(self.colors().link))
                    .on_press(Message::Local(LocalPage::About))
                    .padding([7, 0])
                    .style(quiet),
            );
        if self.account.is_some() {
            sidebar = sidebar.push(
                button(text("Sign out").size(11).color(self.colors().link))
                    .on_press(Message::SignOut)
                    .style(quiet)
                    .padding([7, 0]),
            );
        }
        container(sidebar)
            .width(280)
            .padding([20, 18])
            .style(white_panel)
            .into()
    }
}

fn metric(handle: &image::Handle, count: &str) -> Element<'static, Message> {
    row![
        image(handle.clone()).width(13).height(13),
        text(count.to_string())
            .size(11)
            .font(BOLD)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme_colors(theme).muted)
            })
    ]
    .spacing(4)
    .align_y(alignment::Vertical::Center)
    .into()
}
fn section_title(title: &str) -> Element<'_, Message> {
    container(text(title).size(19).font(BOLD))
        .padding([18, 21])
        .into()
}
fn divider<'a>(height: u16) -> iced::widget::Rule<'a, Theme> {
    iced::widget::horizontal_rule(height).style(|theme: &Theme| iced::widget::rule::Style {
        color: theme_colors(theme).border,
        ..iced::widget::rule::default(theme)
    })
}
fn theme_colors(theme: &Theme) -> preferences::Colors {
    themes::colors(theme)
}
fn white_panel(theme: &Theme) -> container::Style {
    let c = theme_colors(theme);
    container::Style {
        background: Some(c.surface.into()),
        text_color: Some(c.text),
        border: Border {
            color: c.border,
            width: 1.0,
            radius: 7.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
            offset: iced::Vector::new(0.0, 1.0),
            blur_radius: 2.0,
        },
    }
}
fn gradient(top: Color, bottom: Color, angle: f32) -> Background {
    iced::gradient::Linear::new(iced::Radians(angle))
        .add_stop(0.0, top)
        .add_stop(1.0, bottom)
        .into()
}
fn nav_link(label: &str, active: bool, message: Message) -> Element<'_, Message> {
    button(text(label).size(13).font(if active {
        BOLD
    } else {
        Font::with_name("Helvetica")
    }))
    .padding([16, 13])
    .on_press(message)
    .style(move |theme, status| {
        let mut style = top_button(theme, status);
        if active {
            let c = theme_colors(theme);
            style.background = Some(c.nav_bottom.into());
            style.text_color = c.nav_text;
        }
        style
    })
    .into()
}
fn top_button(theme: &Theme, status: button::Status) -> button::Style {
    let c = theme_colors(theme);
    button::Style {
        background: (status == button::Status::Hovered).then_some(c.nav_bottom.into()),
        text_color: c.nav_text,
        border: border::rounded(0),
        ..Default::default()
    }
}
fn primary(theme: &Theme, status: button::Status) -> button::Style {
    let c = theme_colors(theme);
    let accent = theme.palette().primary;
    let background = if status == button::Status::Disabled {
        preferences::mix(accent, c.surface, 0.55).into()
    } else if status == button::Status::Hovered {
        preferences::mix(accent, Color::BLACK, 0.15).into()
    } else {
        gradient(
            preferences::mix(accent, Color::WHITE, 0.12),
            preferences::mix(accent, Color::BLACK, 0.12),
            std::f32::consts::PI,
        )
    };
    let bright = accent.r * 0.299 + accent.g * 0.587 + accent.b * 0.114 > 0.65;
    button::Style {
        background: Some(background),
        text_color: if bright {
            rgb(25, 28, 32)
        } else {
            Color::WHITE
        },
        border: Border {
            color: preferences::mix(accent, Color::BLACK, 0.25),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}
fn bootstrap_button(theme: &Theme, status: button::Status) -> button::Style {
    let c = theme_colors(theme);
    button::Style {
        background: Some(if status == button::Status::Hovered {
            c.inset.into()
        } else {
            gradient(c.surface, c.inset, std::f32::consts::PI)
        }),
        text_color: c.text,
        border: Border {
            color: c.border,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}
fn quiet(theme: &Theme, status: button::Status) -> button::Style {
    let c = theme_colors(theme);
    button::Style {
        background: (status == button::Status::Hovered).then_some(c.inset.into()),
        text_color: if status == button::Status::Disabled {
            c.muted
        } else {
            c.text
        },
        border: border::rounded(3),
        ..Default::default()
    }
}
fn soft_card(theme: &Theme) -> container::Style {
    let c = theme_colors(theme);
    container::Style {
        background: Some(c.inset.into()),
        text_color: Some(c.muted),
        border: border::rounded(3),
        ..Default::default()
    }
}

async fn open_browser(address: &str) -> Result<(), String> {
    let address = address.to_string();
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "macos")]
        let program = "open";
        #[cfg(target_os = "windows")]
        let program = "explorer";
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let program = "xdg-open";
        std::process::Command::new(program)
            .arg(&address)
            .status()
            .map_err(|e| e.to_string())
            .and_then(|s| {
                if s.success() {
                    Ok(())
                } else {
                    Err("Browser failed to open".into())
                }
            })
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn pick_photos() -> Result<Vec<Upload>, String> {
    {
        tokio::task::spawn_blocking(|| {
            use std::io::Read;
            let Some(paths) = rfd::FileDialog::new()
                .set_title("Attach photos to your Ripple post")
                .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp"])
                .pick_files()
            else {
                return Ok(Vec::new());
            };
            if paths.len() > 5 {
                return Err("Choose at most five images.".into());
            }
            let mut uploads = Vec::new();
            for path in paths {
                let file =
                    std::fs::File::open(&path).map_err(|_| "Could not open the selected photo.")?;
                let mut bytes = Vec::new();
                file.take(8 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "Could not read the selected photo.")?;
                if bytes.len() > 8 * 1024 * 1024 {
                    return Err("Each photo must be at most 8 MiB.".into());
                }
                let mime = match ::image::guess_format(&bytes)
                    .map_err(|_| "That file is not a supported photo.")?
                {
                    ::image::ImageFormat::Png => "image/png",
                    ::image::ImageFormat::Jpeg => "image/jpeg",
                    ::image::ImageFormat::Gif => "image/gif",
                    ::image::ImageFormat::WebP => "image/webp",
                    _ => return Err("Choose PNG, JPEG, GIF or WebP images.".into()),
                };
                uploads.push(Upload {
                    name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    bytes,
                    mime: mime.into(),
                });
            }
            Ok(uploads)
        })
        .await
        .map_err(|e| e.to_string())?
    }
}

async fn import_theme() -> Result<Option<themes::ThemePack>, String> {
    {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title("Import Ripple theme")
            .add_filter("Ripple theme", &["json"])
            .pick_file()
            .await
        else {
            return Ok(None);
        };
        let path = file.path().to_path_buf();
        tokio::task::spawn_blocking(move || themes::ThemePack::read(&path))
            .await
            .map_err(|e| e.to_string())?
            .map(Some)
    }
}
async fn export_theme(pack: themes::ThemePack) -> Result<bool, String> {
    {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title("Export Ripple theme")
            .set_file_name("my-ripple-theme.json")
            .add_filter("Ripple theme", &["json"])
            .save_file()
            .await
        else {
            return Ok(false);
        };
        let path = file.path().to_path_buf();
        tokio::task::spawn_blocking(move || {
            let data = serde_json::to_vec_pretty(&pack).map_err(|e| e.to_string())?;
            std::fs::write(path, data).map_err(|_| "Could not export this theme.".to_string())?;
            Ok(true)
        })
        .await
        .map_err(|e| e.to_string())?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_app() -> App {
        let (mut app, _) = App::new();
        app.preferences = Preferences::default();
        app.rebuild_theme();
        app
    }

    fn fixture() -> Page {
        ripple_core::parse_page(
            include_str!("../../../assets/public-preview.html"),
            &Url::parse(ripple_core::ORIGIN).unwrap(),
        )
        .unwrap()
    }
    #[tokio::test]
    async fn social_refresh_keeps_page_order_content_and_returns_no_scroll_command() {
        let mut app = test_app();
        let mut original = fixture();
        original.account = Some("@alice".into());
        app.page = Some(original.clone());
        app.page_number = 3;
        app.publishing = true;
        let mut fetched = original.clone();
        fetched.posts.reverse();
        fetched.posts[0].likes = "999".into();
        let changed_id = fetched.posts[0].id;
        fetched.posts[0].text = "New server content must not move the viewport".into();
        let task = app.update(Message::SocialDone {
            id: app.generation,
            result: Ok(fetched),
        });
        assert!(
            iced_runtime::task::into_stream(task).is_none(),
            "Social refresh must not issue scroll commands"
        );
        let visible = app.page.as_ref().unwrap();
        assert_eq!(app.page_number, 3);
        assert_eq!(
            visible.posts.iter().map(|p| p.id).collect::<Vec<_>>(),
            original.posts.iter().map(|p| p.id).collect::<Vec<_>>()
        );
        assert_eq!(
            visible
                .posts
                .iter()
                .find(|p| p.id == changed_id)
                .unwrap()
                .likes,
            "999"
        );
        assert_eq!(visible.posts[0].text, original.posts[0].text);
        assert!(!app.publishing);
    }
    #[tokio::test]
    async fn older_pages_replace_posts_and_failure_keeps_the_current_page() {
        let mut app = test_app();
        app.page = Some(fixture());
        let mut next = fixture();
        next.posts.drain(0..5);
        let ids = next.posts.iter().map(|p| p.id).collect::<Vec<_>>();
        let _ = app.update(Message::Older);
        let _ = app.update(Message::Loaded {
            id: app.generation,
            page: 2,
            result: Ok(next),
        });
        assert_eq!(app.page_number, 2);
        assert_eq!(
            app.page
                .as_ref()
                .unwrap()
                .posts
                .iter()
                .map(|p| p.id)
                .collect::<Vec<_>>(),
            ids
        );
        let _ = app.update(Message::Older);
        let _ = app.update(Message::Loaded {
            id: app.generation,
            page: 3,
            result: Err("Offline".into()),
        });
        assert_eq!(app.page_number, 2);
        assert_eq!(app.page.as_ref().unwrap().posts.len(), ids.len());
        let _ = app.update(Message::Newer);
        let _ = app.update(Message::Loaded {
            id: app.generation,
            page: 1,
            result: Ok(fixture()),
        });
        assert_eq!(app.page_number, 1);
    }
    #[tokio::test]
    async fn account_settings_with_login_dialog_does_not_expire_the_account() {
        let mut app = test_app();
        app.account = Some("@alice".into());
        let mut page = fixture();
        page.account = Some("@alice".into());
        assert!(page.login_form.is_some());
        let _ = app.update(Message::Loaded {
            id: app.generation,
            page: 1,
            result: Ok(page),
        });
        assert_eq!(app.account.as_deref(), Some("@alice"));
    }
    #[tokio::test]
    async fn late_timeline_response_cannot_replace_a_new_profile_navigation() {
        let mut app = test_app();
        let old = app.generation;
        let _ = app.update(Message::Navigate(Route::Profile("alice".into())));
        let page = ripple_core::parse_page(
            include_str!("../../../assets/public-preview.html"),
            &Url::parse(ripple_core::ORIGIN).unwrap(),
        )
        .unwrap();
        let _ = app.update(Message::Loaded {
            id: old,
            page: 1,
            result: Ok(page),
        });
        assert_eq!(app.route, Route::Profile("alice".into()));
        assert!(app.page.is_none());
        assert!(app.loading);
    }

    #[tokio::test]
    async fn sign_out_discards_account_draft_and_attachments() {
        let mut app = test_app();
        app.account = Some("@alice".into());
        app.draft = text_editor::Content::with_text("Private draft");
        app.uploads.push(Upload {
            name: "photo.png".into(),
            bytes: vec![1, 2, 3],
            mime: "image/png".into(),
        });
        let _ = app.update(Message::SignOut);
        assert!(app.account.is_none());
        assert!(app.draft_text().is_empty());
        assert!(app.uploads.is_empty());
    }

    #[tokio::test]
    async fn composer_counts_utf16_without_the_editors_synthetic_newline() {
        let mut app = test_app();
        app.draft = text_editor::Content::with_text(&"x".repeat(256));
        assert!(app.valid_draft());
        app.draft = text_editor::Content::with_text(&"🙂".repeat(128));
        assert!(app.valid_draft());
        app.draft = text_editor::Content::with_text(&"🙂".repeat(129));
        assert!(!app.valid_draft());
        app.draft = text_editor::Content::new();
        assert!(!app.valid_draft());
        app.uploads.push(Upload {
            name: "photo.png".into(),
            bytes: vec![1],
            mime: "image/png".into(),
        });
        assert!(app.valid_draft());
    }
    #[tokio::test]
    async fn account_forms_keep_multiline_and_checkbox_edits_without_network() {
        let mut app = test_app();
        let html = r#"<a class="brand" href="/"></a><form method="post" action="/settings/save"><textarea name="bio">Old biography</textarea><label><input type="checkbox" name="private" checked>Private profile</label></form>"#;
        let page =
            ripple_core::parse_page(html, &Url::parse("https://ripple.tel/settings").unwrap())
                .unwrap();
        app.sync_account_page(&page);
        app.page = Some(page);
        app.route = Route::Page("https://ripple.tel/settings".into());
        assert_eq!(app.form_drafts[0][0].value, "Old biography");
        let _ = app.update(Message::FieldText(0, 0, "New biography".into()));
        let _ = app.update(Message::FieldCheck(0, 1, false));
        assert_eq!(app.form_drafts[0][0].value, "New biography");
        assert!(!app.form_drafts[0][1].checked);
        let _ = app.view();
    }

    #[tokio::test]
    async fn guest_notification_navigation_opens_sign_in_without_guessing_a_route() {
        let mut app = test_app();
        let original = app.route.clone();
        let _ = app.update(Message::Portal("notif".into()));
        assert!(app.login_open);
        assert_eq!(app.route, original);
        assert!(!app.signing_in);
    }
    #[tokio::test]
    async fn app_preferences_and_about_are_local_and_do_not_change_the_account_route() {
        let mut app = test_app();
        app.route = Route::Profile("alice".into());
        app.account = Some("@alice".into());
        let generation = app.generation;
        let _ = app.update(Message::Local(LocalPage::Appearance));
        let _ = app.update(Message::Appearance(Appearance::Dark));
        assert!(app.colors().surface.r < 0.2);
        assert_eq!(app.generation, generation);
        assert_eq!(app.account.as_deref(), Some("@alice"));
        let _ = app.update(Message::Local(LocalPage::About));
        let _ = app.view();
        let _ = app.update(Message::CloseLocal);
        assert_eq!(app.route, Route::Profile("alice".into()));
        assert!(app.local_page.is_none());
    }
    #[tokio::test]
    async fn accent_updates_theme_and_invalid_hex_does_not_replace_it() {
        let mut app = test_app();
        let _ = app.update(Message::Accent([52, 122, 194]));
        assert_eq!(
            app.theme().palette().primary,
            Color::from_rgb8(52, 122, 194)
        );
        let _ = app.update(Message::AccentText("not a color".into()));
        let _ = app.update(Message::ApplyAccent);
        assert_eq!(app.preferences.accent, [52, 122, 194]);
        assert!(app.preference_error.is_some());
    }
    #[tokio::test]
    async fn unfinished_reply_retains_its_destination_when_navigating() {
        let mut app = test_app();
        app.account = Some("@alice".into());
        app.composition = Composition::Reply(42);
        app.draft = text_editor::Content::with_text("A reply to Alice");
        let _ = app.update(Message::Navigate(Route::Public));
        assert_eq!(app.composition, Composition::Reply(42));
        let _ = app.update(Message::Compose(Composition::Post));
        assert_eq!(app.composition, Composition::Reply(42));
        assert!(app.error.is_some());
        let _ = app.update(Message::DiscardDraft);
        assert!(app.draft_text().is_empty());
        assert_eq!(app.composition, Composition::Post);
    }
}
