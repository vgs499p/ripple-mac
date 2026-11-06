// SPDX-License-Identifier: GPL-2.0-or-later
//! Local app preferences. This file never contains accounts, passwords, or cookies.
use iced::Color;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Appearance {
    #[default]
    Light,
    Dark,
    System,
}
impl std::fmt::Display for Appearance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::System => "System",
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}
impl std::fmt::Display for Density {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Comfortable => "Comfortable",
            Self::Compact => "Compact",
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: String,
    pub imported_themes: Vec<crate::themes::ThemePack>,
    pub appearance: Appearance,
    pub accent: [u8; 3],
    pub density: Density,
    pub text_size: u16,
    pub load_photos: bool,
    pub wallpapers: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: "ripple".into(),
            imported_themes: Vec::new(),
            appearance: Appearance::Light,
            accent: [210, 4, 4],
            density: Density::Comfortable,
            text_size: 100,
            load_photos: true,
            wallpapers: true,
        }
    }
}
impl Preferences {
    pub fn accent_color(&self) -> Color {
        Color::from_rgb8(self.accent[0], self.accent[1], self.accent[2])
    }
    pub fn dark(&self, system_dark: bool) -> bool {
        if let Some(pack) = self.theme_pack() {
            return pack.dark;
        }
        self.appearance == Appearance::Dark
            || (self.appearance == Appearance::System && system_dark)
    }
    pub fn theme_pack(&self) -> Option<&crate::themes::ThemePack> {
        crate::themes::builtins()
            .iter()
            .chain(self.imported_themes.iter())
            .find(|p| p.id == self.theme)
    }
    pub fn colors(&self, system_dark: bool) -> Colors {
        self.theme_pack()
            .map(|p| p.colors())
            .unwrap_or_else(|| colors(self.dark(system_dark), self.accent_color()))
    }
    pub fn accents(&self) -> Vec<crate::themes::Accent> {
        self.theme_pack()
            .map(|p| p.accents.clone())
            .filter(|a| !a.is_empty())
            .unwrap_or_else(crate::themes::ripple_accents)
    }
    pub fn validate(&mut self) {
        self.imported_themes.retain(|p| {
            p.validate().is_ok()
                && p.id != "ripple"
                && !crate::themes::builtins().iter().any(|b| b.id == p.id)
        });
        self.imported_themes.truncate(16);
        if self.theme != "ripple" && self.theme_pack().is_none() {
            self.theme = "ripple".into();
        }
        if self.theme == "ripple" {
            self.accent = match self.accent {
                [216, 57, 67] => [210, 4, 4],
                [52, 122, 194] => [62, 131, 223],
                [35, 139, 112] => [0, 128, 0],
                [134, 82, 199] => [58, 4, 206],
                [204, 132, 35] => [255, 165, 0],
                [207, 76, 145] => [236, 156, 170],
                color => color,
            };
        }
        if ![90, 100, 110, 125].contains(&self.text_size) {
            self.text_size = 100;
        }
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let file = fs::File::open(path).map_err(|_| "Could not read saved app preferences.")?;
        let mut data = Vec::new();
        file.take(524289)
            .read_to_end(&mut data)
            .map_err(|_| "Could not read saved app preferences.")?;
        if data.len() > 524288 {
            return Err("The app preferences file is too large.".into());
        }
        let mut prefs: Self = serde_json::from_slice(&data)
            .map_err(|_| "Saved app preferences could not be read; defaults are active.")?;
        prefs.validate();
        Ok(prefs)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let parent = path
            .parent()
            .ok_or("No app settings folder is available.")?;
        fs::create_dir_all(parent).map_err(|_| "Could not create the app settings folder.")?;
        let temporary = parent.join(format!(".preferences-{}.tmp", std::process::id()));
        let data = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&temporary, data).map_err(|_| "Could not save app preferences.")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
                .map_err(|_| "Could not protect app preferences.")?;
        }
        fs::rename(&temporary, path).map_err(|_| "Could not replace the app preferences file.")?;
        Ok(())
    }
}
pub fn settings_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("RIPPLE_CONFIG_DIR") {
        return PathBuf::from(dir).join("preferences.json");
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library/Application Support/Ripple Native/preferences.json");
    }
    #[cfg(target_os = "windows")]
    if let Some(dir) = std::env::var_os("APPDATA") {
        return PathBuf::from(dir).join("Ripple Native/preferences.json");
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join("ripple-native/preferences.json");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config/ripple-native/preferences.json");
    }
    std::env::temp_dir().join("ripple-native/preferences.json")
}
pub fn parse_accent(value: &str) -> Option<[u8; 3]> {
    let s = value.trim().trim_start_matches('#');
    if s.len() != 6 || !s.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&s[0..2], 16).ok()?,
        u8::from_str_radix(&s[2..4], 16).ok()?,
        u8::from_str_radix(&s[4..6], 16).ok()?,
    ])
}
pub fn hex(accent: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", accent[0], accent[1], accent[2])
}
pub fn system_is_dark() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
            .is_ok_and(|r| {
                r.status.success()
                    && String::from_utf8_lossy(&r.stdout)
                        .trim()
                        .eq_ignore_ascii_case("Dark")
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Colors {
    pub background: Color,
    pub surface: Color,
    pub inset: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub link: Color,
    pub nav_top: Color,
    pub nav_bottom: Color,
    pub nav_text: Color,
    pub success: Color,
    pub danger: Color,
}
pub fn colors(dark: bool, accent: Color) -> Colors {
    let rgb = Color::from_rgb8;
    if dark {
        Colors {
            background: rgb(22, 25, 30),
            surface: rgb(34, 39, 47),
            inset: rgb(43, 49, 59),
            border: rgb(60, 68, 81),
            text: rgb(234, 237, 242),
            muted: rgb(165, 175, 190),
            link: mix(accent, Color::WHITE, 0.35),
            nav_top: rgb(42, 46, 53),
            nav_bottom: rgb(21, 24, 29),
            nav_text: rgb(202, 204, 207),
            success: rgb(57, 145, 113),
            danger: rgb(210, 4, 4),
        }
    } else {
        Colors {
            background: rgb(238, 238, 238),
            surface: Color::WHITE,
            inset: rgb(245, 245, 245),
            border: rgb(221, 221, 221),
            text: rgb(64, 64, 64),
            muted: rgb(128, 128, 128),
            link: rgb(0, 105, 214),
            nav_top: rgb(42, 46, 53),
            nav_bottom: rgb(21, 24, 29),
            nav_text: rgb(202, 204, 207),
            success: rgb(57, 145, 113),
            danger: rgb(210, 4, 4),
        }
    }
}
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    }
}

/// Recolors only the red branding pixels; highlights, outlines and alpha survive.
pub fn tint(
    bytes: &[u8],
    accent: [u8; 3],
    header: bool,
) -> Result<iced::widget::image::Handle, String> {
    let mut pixels = image::load_from_memory(bytes)
        .map_err(|e| e.to_string())?
        .into_rgba8();
    let (h, _, _) = hsl(accent.map(|v| v as f32 / 255.0));
    let (_, accent_sat, _) = hsl(accent.map(|v| v as f32 / 255.0));
    for pixel in pixels.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        if r as i16 - g as i16 > 5 && r as i16 - b as i16 > 5 {
            let (_, s, l) = hsl([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);
            let mut rgb = from_hsl(h, s * accent_sat, l);
            if header {
                let light = luminance(rgb);
                if light > 0.18 {
                    let scale = (0.18 / light).powf(1.0 / 2.2) * 0.97;
                    rgb = rgb.map(|v| v * scale);
                }
            }
            pixel.0 = [
                (rgb[0] * 255.0).round() as u8,
                (rgb[1] * 255.0).round() as u8,
                (rgb[2] * 255.0).round() as u8,
                a,
            ];
        }
    }
    Ok(iced::widget::image::Handle::from_rgba(
        pixels.width(),
        pixels.height(),
        pixels.into_raw(),
    ))
}
fn hsl([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let l = (max + min) / 2.0;
    if d < 0.0001 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}
fn from_hsl(h: f32, s: f32, l: f32) -> [f32; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let rgb = match (h * 6.0) as u8 {
        0 => [c, x, 0.0],
        1 => [x, c, 0.0],
        2 => [0.0, c, x],
        3 => [0.0, x, c],
        4 => [x, 0.0, c],
        _ => [c, 0.0, x],
    };
    rgb.map(|v| v + m)
}
fn luminance(rgb: [f32; 3]) -> f32 {
    let c = rgb.map(|v| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_round_trip_and_validation() {
        let dir = std::env::temp_dir().join(format!("ripple-prefs-test-{}", std::process::id()));
        let path = dir.join("preferences.json");
        let prefs = Preferences {
            accent: [62, 131, 223],
            appearance: Appearance::Dark,
            density: Density::Compact,
            ..Default::default()
        };
        prefs.save(&path).unwrap();
        assert_eq!(Preferences::load(&path).unwrap(), prefs);
        fs::write(&path, r#"{"text_size":900}"#).unwrap();
        assert_eq!(Preferences::load(&path).unwrap().text_size, 100);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn accents_reject_invalid_values() {
        assert_eq!(parse_accent("#347ac2"), Some([52, 122, 194]));
        assert!(parse_accent("#123").is_none());
        assert!(parse_accent("zzzzzz").is_none());
        assert!(parse_accent("💙💙💙").is_none());
    }
    #[test]
    fn tint_preserves_white_black_alpha_and_changes_red() {
        let mut img = image::RgbaImage::new(3, 1);
        for (x, p) in [[255, 255, 255, 255], [0, 0, 0, 128], [220, 25, 25, 255]]
            .into_iter()
            .enumerate()
        {
            img.put_pixel(x as u32, 0, image::Rgba(p));
        }
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let handle = tint(bytes.get_ref(), [30, 90, 220], false).unwrap();
        if let iced::widget::image::Handle::Rgba { pixels, .. } = handle {
            assert_eq!(&pixels[0..4], &[255, 255, 255, 255]);
            assert_eq!(&pixels[4..8], &[0, 0, 0, 128]);
            assert!(pixels[10] > pixels[8]);
        } else {
            panic!("expected pixels");
        }
    }
}
