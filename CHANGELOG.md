# 0.4.0-beta.5

- Keep the thread selected after a reply and show the correct reply/quote success message.

# 0.4.0-beta.4

- Fix replies being reported as unsuccessful when Ripple only shows them in the conversation thread. Confirmation now checks the thread used for the reply.

# 0.4.0-beta.3

- Save confirmed sign-ins in macOS Keychain and restore them on launch.
- Keep passwords out of persistent storage; honor cookie paths and server expiry.
- Save cookie rotations automatically without rewriting unchanged sessions.
- Sign out removes the saved session and prevents late requests from restoring it.
- Display Keychain errors instead of claiming that sign-in was saved or removed.
- Identify Ripple Native, its version and the running macOS version in the user agent.
- Include a server-side adapter example for labeling Ripple Native sessions.

# 0.4.0-beta.2

- Likes/reposts patch counts and actions in place, preserving the visible posts and scroll position.
- Older/Newer buttons switch server pages instead of appending into a growing feed; refresh stays on the current page.
- Account settings and login dialogs no longer falsely clear a valid signed-in account.
- macOS About menu opens the same native About page as App Settings.
- Application source licensed GPL-2.0-or-later; Mac binary uses the GPL-3.0-or-later option for dependency compatibility.
- Homebrew cask and release packaging instructions.

# 0.4.0-beta.1 — First Mac beta

- Ripple's exact seven accent swatches; accent-colored logo and masthead.
- Complete native theme packs: Ripple, Breeze Light/Dark, Nord, four Catppuccin flavors and three Rosé Pine variants.
- Import, export and remove custom JSON themes; persistent local preferences.
- Native App Settings and About, text size, density, wallpaper and photo controls.
- Real Users directory and pagination; correct Following and Notifications routes.
- Profile About Me details and support for Ripple's profile `like.php` links.
- Cached themes/post views and protection for unfinished reply/quote drafts.

This build targets Apple silicon on macOS 11+. Sign-in and public posting were demonstrated by the user. Public read-only pages were checked live. Replies, quotes, uploads, social actions and settings saves remain beta features awaiting end-to-end verification. As of Beta 3, sessions are saved in Keychain and survive closing the app. Account editor parity, animated media, arbitrary profile CSS, and legacy-platform builds remain unfinished.
