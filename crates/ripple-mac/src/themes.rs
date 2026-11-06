// SPDX-License-Identifier: GPL-2.0-or-later
//! Data-only native themes. Packs cannot execute code or reference remote assets.
use crate::preferences::{self, Colors, Preferences};
use iced::{
    theme::{
        palette::{Extended, Pair},
        Palette,
    },
    Color, Theme,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{io::Read, path::Path, sync::OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hex(pub [u8; 3]);
impl Hex {
    pub fn color(self) -> Color {
        Color::from_rgb8(self.0[0], self.0[1], self.0[2])
    }
}
impl Serialize for Hex {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&preferences::hex(self.0))
    }
}
impl<'de> Deserialize<'de> for Hex {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        preferences::parse_accent(&s)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom("Colors must be six-digit hex values"))
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tokens {
    pub background: Hex,
    pub surface: Hex,
    pub inset: Hex,
    pub border: Hex,
    pub text: Hex,
    pub muted: Hex,
    pub link: Hex,
    pub nav_top: Hex,
    pub nav_bottom: Hex,
    pub nav_text: Hex,
    pub accent: Hex,
    pub success: Hex,
    pub danger: Hex,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accent {
    pub name: String,
    pub color: Hex,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemePack {
    pub version: u8,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    pub dark: bool,
    pub colors: Tokens,
    #[serde(default)]
    pub accents: Vec<Accent>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub id: String,
    pub name: String,
}
impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}
impl ThemePack {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("This theme format version is not supported.".into());
        }
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || self.name.trim().is_empty()
            || self.name.len() > 80
            || self.author.len() > 160
            || self.accents.len() > 24
            || self
                .accents
                .iter()
                .any(|a| a.name.trim().is_empty() || a.name.len() > 40)
        {
            return Err("The theme has invalid names or too many accents.".into());
        }
        Ok(())
    }
    pub fn read(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|_| "Could not open the theme file.")?;
        let mut bytes = Vec::new();
        file.take(32769)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read the theme.")?;
        if bytes.len() > 32768 {
            return Err("Choose a theme file smaller than 32 KiB.".into());
        }
        let theme: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("Invalid theme: {e}"))?;
        theme.validate()?;
        if theme.id == "ripple" || builtins().iter().any(|p| p.id == theme.id) {
            return Err("Give imported themes their own id, such as my-mocha.".into());
        }
        Ok(theme)
    }
    pub fn colors(&self) -> Colors {
        let c = &self.colors;
        Colors {
            background: c.background.color(),
            surface: c.surface.color(),
            inset: c.inset.color(),
            border: c.border.color(),
            text: c.text.color(),
            muted: c.muted.color(),
            link: c.link.color(),
            nav_top: c.nav_top.color(),
            nav_bottom: c.nav_bottom.color(),
            nav_text: c.nav_text.color(),
            success: c.success.color(),
            danger: c.danger.color(),
        }
    }
}
pub fn builtins() -> &'static [ThemePack] {
    static PACKS: OnceLock<Vec<ThemePack>> = OnceLock::new();
    PACKS.get_or_init(|| {
        [
            include_str!("../../../assets/themes/breeze-light.json"),
            include_str!("../../../assets/themes/breeze-dark.json"),
            include_str!("../../../assets/themes/nord.json"),
            include_str!("../../../assets/themes/catppuccin-latte.json"),
            include_str!("../../../assets/themes/catppuccin-frappe.json"),
            include_str!("../../../assets/themes/catppuccin-macchiato.json"),
            include_str!("../../../assets/themes/catppuccin-mocha.json"),
            include_str!("../../../assets/themes/rose-pine.json"),
            include_str!("../../../assets/themes/rose-pine-moon.json"),
            include_str!("../../../assets/themes/rose-pine-dawn.json"),
        ]
        .iter()
        .map(|s| serde_json::from_str(s).expect("bundled theme is valid"))
        .collect()
    })
}
/// Exact swatches from https://ripple.tel/css/misc.css, inspected November 6, 2026.
pub fn ripple_accents() -> Vec<Accent> {
    [
        ("Red", [210, 4, 4]),
        ("Orange", [255, 165, 0]),
        ("Green", [0, 128, 0]),
        ("Pink", [236, 156, 170]),
        ("Indigo", [58, 4, 206]),
        ("Blue", [62, 131, 223]),
        ("Graphite", [42, 46, 53]),
    ]
    .into_iter()
    .map(|(name, c)| Accent {
        name: name.into(),
        color: Hex(c),
    })
    .collect()
}
pub fn choices(prefs: &Preferences) -> Vec<Choice> {
    std::iter::once(Choice {
        id: "ripple".into(),
        name: "Ripple".into(),
    })
    .chain(
        builtins()
            .iter()
            .chain(prefs.imported_themes.iter())
            .map(|p| Choice {
                id: p.id.clone(),
                name: p.name.clone(),
            }),
    )
    .collect()
}
pub fn build(prefs: &Preferences, system_dark: bool) -> Theme {
    let c = prefs.colors(system_dark);
    let dark = prefs.dark(system_dark);
    Theme::custom_with_fn(
        prefs.theme.clone(),
        Palette {
            background: c.background,
            text: c.text,
            primary: prefs.accent_color(),
            success: c.success,
            danger: c.danger,
        },
        move |p| {
            let mut e = Extended::generate(p);
            e.is_dark = dark;
            e.background.base = Pair {
                color: c.background,
                text: c.text,
            };
            e.background.weak = Pair {
                color: c.surface,
                text: c.text,
            };
            e.background.strong = Pair {
                color: c.inset,
                text: c.text,
            };
            e.secondary.base = Pair {
                color: c.border,
                text: c.muted,
            };
            e.secondary.weak = Pair {
                color: c.nav_top,
                text: c.nav_text,
            };
            e.secondary.strong = Pair {
                color: c.nav_bottom,
                text: c.nav_text,
            };
            e.primary.strong = Pair {
                color: c.link,
                text: c.text,
            };
            e
        },
    )
}
pub fn colors(theme: &Theme) -> Colors {
    let p = theme.palette();
    let e = theme.extended_palette();
    Colors {
        background: p.background,
        surface: e.background.weak.color,
        inset: e.background.strong.color,
        border: e.secondary.base.color,
        text: p.text,
        muted: e.secondary.base.text,
        link: e.primary.strong.color,
        nav_top: e.secondary.weak.color,
        nav_bottom: e.secondary.strong.color,
        nav_text: e.secondary.weak.text,
        success: p.success,
        danger: p.danger,
    }
}
pub fn export(prefs: &Preferences, system_dark: bool) -> ThemePack {
    let c = prefs.colors(system_dark);
    let hex = |c: Color| {
        Hex([
            (c.r * 255.0).round() as u8,
            (c.g * 255.0).round() as u8,
            (c.b * 255.0).round() as u8,
        ])
    };
    ThemePack {
        version: 1,
        id: "my-ripple-theme".into(),
        name: "My Ripple theme".into(),
        author: String::new(),
        dark: prefs.dark(system_dark),
        colors: Tokens {
            background: hex(c.background),
            surface: hex(c.surface),
            inset: hex(c.inset),
            border: hex(c.border),
            text: hex(c.text),
            muted: hex(c.muted),
            link: hex(c.link),
            nav_top: hex(c.nav_top),
            nav_bottom: hex(c.nav_bottom),
            nav_text: hex(c.nav_text),
            accent: Hex(prefs.accent),
            success: hex(c.success),
            danger: hex(c.danger),
        },
        accents: prefs.accents(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packs_round_trip_and_tokens_reach_native_widgets() {
        for pack in builtins() {
            pack.validate().unwrap();
            let prefs = Preferences {
                theme: pack.id.clone(),
                accent: pack.colors.accent.0,
                ..Default::default()
            };
            let theme = build(&prefs, false);
            let c = colors(&theme);
            assert_eq!(c.surface, pack.colors.surface.color());
            assert_eq!(c.text, pack.colors.text.color());
            assert_eq!(theme.extended_palette().is_dark, pack.dark);
            assert_eq!(
                &serde_json::from_str::<ThemePack>(&serde_json::to_string(pack).unwrap()).unwrap(),
                pack
            );
        }
    }
    #[test]
    fn data_only_packs_reject_scripts_and_invalid_colors() {
        let text = include_str!("../../../assets/themes/catppuccin-mocha.json");
        assert!(serde_json::from_str::<ThemePack>(
            &text.replace("#1e1e2e", "url(https://example.com)")
        )
        .is_err());
        let injected = text.replacen('{', "{\"script\":\"run()\",", 1);
        assert!(serde_json::from_str::<ThemePack>(&injected).is_err());
        assert_eq!(ripple_accents()[0].color.0, [210, 4, 4]);
    }
    #[test]
    fn exported_theme_imports_and_survives_preferences_reload() {
        let dir = std::env::temp_dir().join(format!("ripple-theme-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let prefs = Preferences {
            theme: "nord".into(),
            accent: [136, 192, 208],
            ..Default::default()
        };
        let pack = export(&prefs, false);
        let path = dir.join("theme.json");
        std::fs::write(&path, serde_json::to_vec(&pack).unwrap()).unwrap();
        let loaded = ThemePack::read(&path).unwrap();
        assert_eq!(loaded.colors.surface, pack.colors.surface);
        let installed = Preferences {
            theme: loaded.id.clone(),
            accent: loaded.colors.accent.0,
            imported_themes: vec![loaded],
            ..Default::default()
        };
        installed.save(&dir.join("preferences.json")).unwrap();
        let restored = Preferences::load(&dir.join("preferences.json")).unwrap();
        assert_eq!(restored, installed);
        assert_eq!(restored.colors(false).surface, prefs.colors(false).surface);
        std::fs::write(&path, vec![b' '; 32769]).unwrap();
        assert!(ThemePack::read(&path).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
