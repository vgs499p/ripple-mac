# Ripple Native — Desktop Beta 7

A compiled Rust client that connects directly to https://ripple.tel over HTTPS. There is no intermediary service, embedded website, or public API dependency. The interface keeps Ripple's red, charcoal and white identity with desktop navigation and compact post lists.

[Download Desktop Beta 7](https://github.com/vgs499p/ripple-mac/releases/tag/v0.4.0-beta.7) · [Homebrew installation](docs/HOMEBREW.md)

See [desktop platform builds and packaging](docs/PLATFORMS.md) for Intel macOS, Linux, Windows and BSD status.

## Run

Open the supplied `Ripple Beta.app`. The binary targets Apple silicon, declares macOS 11 or later, and is locally ad-hoc signed. It is not notarized for public distribution. Only the current build machine has been tested.

From source with a current Rust toolchain:

```sh
cargo run --locked -p ripple-mac
sh scripts/bundle-macos.sh
```

The script writes `dist/Ripple Beta.app`; `CARGO_TARGET_DIR` and `RIPPLE_DIST_DIR` override the build and output directories.

## Features and verification status

Implemented:

- Public and Following timelines, notifications, manual refresh, older posts, avatars, photos, search and hashtags.
- The real paginated Users directory, verified badges and profile About Me details.
- Local Light/Dark/System appearance, full theme packs, exact Ripple accent presets and custom colors, recolored Ripple branding, density, interface size, wallpaper/photo preferences, and an About page.
- Profile details, wallpapers and banners, followers/following lists, and thread navigation.
- Sign-in through Ripple's actual login form, with sessions saved in macOS Keychain.
- Likes, reposts, follow/unfollow through action links discovered in the current page.
- Public posts, replies and quotes through discovered forms, including hidden fields.
- Native photo picker, PNG/JPEG/GIF/WebP attachments, upload size limits and attachment removal.
- Native notifications and account/profile editors using navigation links and forms discovered after sign-in.
- Settings controls for text, multiline biographies, passwords, checkboxes, choices and image files. Saves re-fetch the form and hidden tokens. Unsupported forms remain unavailable rather than using guessed endpoints.
- Low data mode, bounded image memory, cached post views, and keyboard shortcuts.

Live checks on November 6, 2026 passed public timeline, avatar, thread, profile, search, feed pagination, Users directory and directory pagination. The user's own screenshot confirms successful sign-in and public posting in the native client. The profile alias redirects to HTTP, so profiles use the observed canonical HTTPS `/username` route.

Helium inspection and the user's screenshots confirmed `?tlmode=following`, `?tlmode=mentions`, the paginated Users directory, profile tabs, and Account/Edit Profile/Security/Privacy screens. The website's Trending destination redirects to “Soon!”; the native Trending view instead shows the real sidebar topics. Memories opened a blank page. Security labels 2FA as coming soon.

The reply composer opens in the selected thread. Reply confirmation now checks that thread rather than only the public timeline; a live reply was not sent as part of this fix. Quotes, uploads, social actions and settings saves remain unverified end to end. The agent's inspection was read-only. The generic account editor renders supported discovered forms; full parity with Ripple's account panels is still pending. Custom profile CSS is not executed.

The bundled public snapshot and one profile snapshot were captured on November 5, 2026 at approximately 20:36 America/Fortaleza. They are explicitly labeled as offline previews. Photos and avatars that were not captured show placeholders. Snapshot data cannot be used to submit social actions or settings changes. Refresh reconnects to the real site.

See [theme pack instructions](docs/THEMES.md) for Breeze, Nord, Catppuccin, Rosé Pine and importing/exporting custom palettes. [Beta 7 release notes](CHANGELOG.md) describe the current scope.

Session labels and the client User-Agent format are documented in [USER_AGENT.md](docs/USER_AGENT.md).

## Controls

- `Cmd+R`: refresh
- `Cmd+F`: focus search
- `Cmd+L`: sign-in screen
- `Cmd+,`: app settings
- `Cmd+Q`: quit
- Escape: leave sign-in or app settings

Click an author/avatar to open their profile, a hashtag to search, or a photo to open its original in the default browser. Likes and reposts require sign-in. Click Reply or Quote to open the corresponding composer. Your sign-in survives quitting and restarting. Sign out removes the saved session from Keychain. Drafts are not persisted to disk; navigating keeps an unfinished reply or quote attached to its destination. Local app preferences are stored in `~/Library/Application Support/Ripple Native/preferences.json`, separately from the account. Passwords are never saved. Session cookies and the account handle are saved in Keychain, separate from preferences. Ripple can still expire or revoke a session.

## Rendering and limits

The default desktop build uses Iced's accelerated renderer (Metal on the tested Mac). Static image handles and post widget trees are cached. The theme and post widget trees are cached. Frame timing varies with display size, theme and media; performance measurements are local development checks, not a guarantee for other devices.

The optional software renderer can be built separately:

```sh
cargo run --locked -p ripple-mac --no-default-features --features software
```

The software renderer was noticeably slower on the tested Retina display; it is retained for experimentation rather than recommended for the Mac build.

Network/image work runs off the UI thread. Up to four image pipelines run concurrently. Images have an 8 MiB response limit, 64 MiB decode limit per image, and a decoded edge of at most 1000 pixels (96 pixels for avatars). The decoded feed-image cache has a 64 MiB budget, with current profile appearance images and small avatars pinned. Only the first 24 attachment photos preload; other photos load on demand. Timeline pages replace the visible posts instead of appending indefinitely. Likes and reposts refresh counts in place without resetting the viewport. There is no periodic polling, telemetry or automatic update service.

Posts render as text and separate links. Animated pictures show a still frame. Video, native notification delivery while the app is closed, rich HTML/CSS profile customization, account recovery and browser challenges are outside this prototype. Background images and profile banners are supported; arbitrary profile JavaScript/CSS is not executed.

## Source layout

```
crates/ripple-core/  HTTP, sessions, models, HTML/form parsing, live read-only probe
crates/ripple-mac/   Desktop interface, image pipeline, photo picker
assets/             Branding, bundled public preview, UI assets
scripts/            macOS app packaging
```

All session requests and redirects remain on HTTPS and on Ripple's origin. Media requests do not carry account cookies. Account editor routes and action links must be discovered in the page; arbitrary account URLs are rejected. Forms preserve fresh hidden fields and declared controls, and HTTP mutations are never retried automatically. Ambiguous publishing leaves the draft intact and asks the user to check Ripple before retrying.

## Validate

```sh
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo run --locked -p ripple-core --example probe
```

The probe reads public pages and one avatar; it never signs in or performs social actions. It will fail while Ripple is offline. Tests use synthetic markup and captured public structures; passing tests does not establish that unobserved authenticated markup matches.

Developer-only `qa` builds provide F8 to capture the window beside the app bundle and F9 for a short automated scrolling measurement. These controls are excluded from the supplied normal build.

The application source is licensed under GPL-2.0-or-later. The Mac binary is distributed under GPL-3.0-or-later for compatibility with Apache-2.0 dependencies. See LICENSE, COPYING-GPL-3.0 and THIRD-PARTY-NOTICES.txt. Ripple's name, logo, website assets and captured public content belong to their respective owners; the source license does not relicense those assets. This beta is for the owner's review and is not an official Ripple release. Bundled palette data retains the licenses documented in docs/THEMES.md.
