# Theme packs

Open **App → Appearance → Theme pack** (or Cmd+,). Beta 2 includes Ripple, Breeze Light/Dark, Nord, Catppuccin Latte/Frappé/Macchiato/Mocha, and Rosé Pine/Main/Moon/Dawn. Ripple keeps its Light/Dark/System setting; other packs declare their own color mode.

Ripple's seven accent swatches use the exact values in its live `misc.css`: Red `#D20404`, Orange `#FFA500`, Green `#008000`, Pink `#EC9CAA`, Indigo `#3A04CE`, Blue `#3E83DF`, Graphite `#2A2E35`. Other packs supply their own accent palettes. The logo, masthead and primary controls follow the selected accent; links use the theme's link token.

## Create a theme

1. Choose a theme near the look you want, then **Export current theme…**.
2. Edit its JSON colors and give it a unique `id` and `name`.
3. Choose **Import theme…**. The imported pack is applied immediately and appears in the picker. Reimporting its id updates it. Use **Remove this imported theme** to uninstall it.

The files in `assets/themes/` are working examples. Hex colors use six RGB digits. Format:

```json
{
  "version": 1,
  "id": "my-ripple-theme",
  "name": "My Ripple theme",
  "author": "",
  "dark": true,
  "colors": {
    "background": "#191724",
    "surface": "#1F1D2E",
    "inset": "#26233A",
    "border": "#6E6A86",
    "text": "#E0DEF4",
    "muted": "#908CAA",
    "link": "#9CCFD8",
    "nav_top": "#1F1D2E",
    "nav_bottom": "#191724",
    "nav_text": "#E0DEF4",
    "accent": "#EBBCBA",
    "success": "#9CCFD8",
    "danger": "#EB6F92"
  },
  "accents": [{"name": "Rose", "color": "#EBBCBA"}]
}
```

Each pack declares all 13 tokens. Theme changes update cached widget palettes and branding; parsing and file access do not run per frame. The pack is copied into local preferences, so moving the original file does not break it. Preferences and theme packs contain no session cookies or credentials.

Packs are data-only: no CSS, JavaScript, remote assets or network requests. Imports are limited to 32 KiB, 24 accents and 16 installed custom packs. Unknown properties, invalid colors, unsupported versions and reserved ids are rejected. There are no new runtime dependencies for themes.

## Palette credits

These are native Ripple adaptations of the upstream colors, retaining Ripple's layout and chrome.

- [Catppuccin](https://github.com/catppuccin/palette): palette 1.8.0, MIT; `CATPPUCCIN-LICENSE`.
- [KDE Breeze](https://github.com/KDE/breeze/tree/master/colors): Breeze Light/Dark color roles, LGPL-2.0-or-later; `BREEZE-LICENSE` and original files under `upstream/`, with their contributor notices.
- [Nord](https://github.com/nordtheme/nord): MIT; `NORD-LICENSE`.
- [Rosé Pine](https://github.com/rose-pine/rose-pine-palette): MIT; `ROSE-PINE-LICENSE`.

The palette JSON files, notices and licenses are included in the source and app Resources/Themes folder. Application code is GPL-2.0-or-later; upstream palette data retains its respective license.
