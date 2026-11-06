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

This build targets Apple silicon on macOS 11+. Sign-in and public posting were demonstrated by the user. Public read-only pages were checked live. Replies, quotes, uploads, social actions and settings saves remain beta features awaiting end-to-end verification. Session cookies stay in memory, so closing the app signs out. Account editor parity, animated media, arbitrary profile CSS, and legacy-platform builds remain unfinished.
