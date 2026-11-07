#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
// SPDX-License-Identifier: GPL-2.0-or-later
#[cfg(target_os = "macos")]
mod mac_menu;
mod preferences;
mod secure_session;
mod snapshot_media;
mod themes;
mod ui;

fn main() -> iced::Result {
    iced::application("Ripple", ui::App::update, ui::App::view)
        .subscription(ui::App::subscription)
        .theme(ui::App::theme)
        .scale_factor(ui::App::scale_factor)
        .default_font(iced::Font::with_name("Helvetica"))
        .window(iced::window::Settings {
            size: iced::Size::new(1180.0, 840.0),
            min_size: Some(iced::Size::new(780.0, 580.0)),
            ..Default::default()
        })
        .centered()
        .run_with(ui::App::new)
}
