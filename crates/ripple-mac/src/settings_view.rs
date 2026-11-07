// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
use iced::widget::column;

impl App {
    pub(super) fn local_view(&self, page: LocalPage) -> Element<'_, Message> {
        let c = self.colors();
        let mut navigation = column![
            text("APP SETTINGS").size(11).font(BOLD).color(c.muted),
            Space::with_height(7)
        ]
        .spacing(6);
        for (label, target) in [
            ("Appearance", LocalPage::Appearance),
            ("General", LocalPage::General),
            ("About Ripple", LocalPage::About),
        ] {
            let selected =
                page == target || (page == LocalPage::License && target == LocalPage::About);
            let accent = self.preferences.accent_color();
            navigation = navigation.push(
                button(text(label).size(14).font(if selected {
                    BOLD
                } else {
                    Font::with_name("Helvetica")
                }))
                .width(Length::Fill)
                .padding([11, 12])
                .on_press(Message::Local(target))
                .style(move |theme, status| button::Style {
                    background: (selected || status == button::Status::Hovered).then_some(
                        preferences::mix(accent, theme_colors(theme).surface, 0.88).into(),
                    ),
                    text_color: if selected {
                        theme_colors(theme).link
                    } else {
                        theme_colors(theme).text
                    },
                    border: border::rounded(5),
                    ..Default::default()
                }),
            );
        }
        navigation = navigation.push(Space::with_height(16)).push(
            button(text("< Back to Ripple").size(12))
                .on_press(Message::CloseLocal)
                .padding([10, 12])
                .style(quiet),
        );
        let mut content = Column::new().spacing(18).width(Length::Fill);
        if let Some(error) = &self.preference_error {
            content = content.push(self.banner(error, true));
        }
        match page {
            LocalPage::Appearance => {
                content = content.push(section(
                    "Appearance",
                    "Keep Ripple familiar. Make the details your own.",
                    c,
                ));
                let options = themes::choices(&self.preferences);
                if self
                    .preferences
                    .imported_themes
                    .iter()
                    .any(|p| p.id == self.preferences.theme)
                {
                    content = content.push(
                        button(text("Remove this imported theme").size(12))
                            .on_press(Message::RemoveTheme)
                            .style(quiet)
                            .padding(8),
                    );
                }
                let selected = options
                    .iter()
                    .find(|c| c.id == self.preferences.theme)
                    .cloned();
                content = content.push(card(
                    column![
                        text("Theme pack").size(15).font(BOLD),
                        text("Complete palettes for the timeline, navigation, text, and controls.")
                            .size(12)
                            .color(c.muted),
                        pick_list(options, selected, Message::ThemeSelected)
                            .padding(11)
                            .text_size(14)
                            .width(Length::Fill),
                        row![
                            button(text("Import theme...").size(12))
                                .on_press(Message::ImportTheme)
                                .padding([9, 13])
                                .style(bootstrap_button),
                            button(text("Export current theme...").size(12))
                                .on_press(Message::ExportTheme)
                                .padding([9, 13])
                                .style(bootstrap_button)
                        ]
                        .spacing(10),
                        text("JSON theme packs are stored locally. They work offline.")
                            .size(11)
                            .color(c.muted)
                    ]
                    .spacing(13),
                ));
                let mut modes = row![].spacing(10);
                for mode in [Appearance::Light, Appearance::Dark, Appearance::System] {
                    let active = self.preferences.appearance == mode;
                    let accent = self.preferences.accent_color();
                    modes = modes.push(
                        button(
                            container(
                                column![
                                    text(mode.to_string()).size(14).font(BOLD),
                                    text(if active { "Selected" } else { "Choose" })
                                        .size(11)
                                        .color(c.muted)
                                ]
                                .spacing(8),
                            )
                            .width(Length::Fill)
                            .padding(13),
                        )
                        .width(Length::Fill)
                        .padding(0)
                        .on_press(Message::Appearance(mode))
                        .style(move |theme, _| button::Style {
                            background: Some(theme_colors(theme).surface.into()),
                            text_color: theme_colors(theme).text,
                            border: Border {
                                color: if active {
                                    accent
                                } else {
                                    theme_colors(theme).border
                                },
                                width: if active { 2.0 } else { 1.0 },
                                radius: 6.0.into(),
                            },
                            ..Default::default()
                        }),
                    );
                }
                if self.preferences.theme == "ripple" {
                    content = content.push(card(
                        column![
                            text("Color mode").size(15).font(BOLD),
                            modes,
                            text("System appearance follows macOS; other desktops currently use the light appearance.")
                                .size(12)
                                .color(c.muted)
                        ]
                        .spacing(13),
                    ));
                } else {
                    content = content.push(
                        text(if self.preferences.dark(self.system_dark) {
                            "This theme uses a dark palette."
                        } else {
                            "This theme uses a light palette."
                        })
                        .size(12)
                        .color(c.muted),
                    );
                }
                let presets = self.preferences.accents();
                let mut palette = Column::new().spacing(9);
                for chunk in presets.chunks(3) {
                    let mut line = row![].spacing(9);
                    for preset in chunk {
                        let accent = preset.color.0;
                        let chosen = self.preferences.accent == accent;
                        let color = Color::from_rgb8(accent[0], accent[1], accent[2]);
                        let swatch =
                            container(Space::new(16, 16)).style(move |_| container::Style {
                                background: Some(color.into()),
                                border: border::rounded(8),
                                ..Default::default()
                            });
                        line = line.push(
                            button(
                                row![swatch, text(preset.name.clone()).size(12)]
                                    .spacing(8)
                                    .align_y(alignment::Vertical::Center),
                            )
                            .width(Length::Fill)
                            .padding([10, 11])
                            .on_press(Message::Accent(accent))
                            .style(move |theme, status| {
                                let mut style = bootstrap_button(theme, status);
                                if chosen {
                                    style.border.color = color;
                                    style.border.width = 2.0;
                                }
                                style
                            }),
                        );
                    }
                    palette = palette.push(line);
                }
                let valid = preferences::parse_accent(&self.accent_text).is_some();
                let custom = row![
                    text_input("#D20404", &self.accent_text)
                        .on_input(Message::AccentText)
                        .on_submit(Message::ApplyAccent)
                        .padding(10)
                        .size(13)
                        .width(Length::Fill),
                    button(text("Apply").size(12).font(BOLD))
                        .on_press_maybe(valid.then_some(Message::ApplyAccent))
                        .padding([10, 17])
                        .style(primary)
                ]
                .spacing(10);
                content = content.push(card(
                    column![
                        text("Accent color").size(15).font(BOLD),
                        text(if self.preferences.theme == "ripple" { "Exact swatches from Ripple. The logo, header, and buttons follow your accent." } else { "Accents supplied by this theme. The logo, header, and buttons follow your accent." })
                            .size(12)
                            .color(c.muted)
                            .line_height(1.4),
                        palette,
                        custom
                    ]
                    .spacing(13),
                ));
                content=content.push(card(column![text("Reading").size(15).font(BOLD),
                    row![column![text("Post spacing").size(13).font(BOLD),text("A little breathing room, or a denser timeline.").size(11).color(c.muted)].spacing(5),Space::with_width(Length::Fill),pick_list([Density::Comfortable,Density::Compact],Some(self.preferences.density),Message::Density).padding(9).text_size(12)].spacing(12).align_y(alignment::Vertical::Center),
                    row![column![text("Interface size").size(13).font(BOLD),text("100 is the standard size. Larger values enlarge text and controls.").size(11).color(c.muted)].spacing(5),Space::with_width(Length::Fill),pick_list([90_u16,100,110,125],Some(self.preferences.text_size),Message::TextSize).padding(9).text_size(12)].spacing(12).align_y(alignment::Vertical::Center),
                    checkbox("Show profile wallpapers",self.preferences.wallpapers).on_toggle(Message::Wallpapers).size(18).text_size(13),
                ].spacing(18)));
            }
            LocalPage::General => {
                content = content.push(section(
                    "General",
                    "Preferences for this device, separate from your Ripple account.",
                    c,
                ));
                content=content.push(card(column![text("Photos and data").size(15).font(BOLD),
                    checkbox("Load post photos automatically",!self.low_data).on_toggle(|load|Message::LowData(!load)).size(18).text_size(13),
                    text("When turned off, avatars remain visible and photos load when you choose them. Profile wallpapers also pause in low data mode.").size(12).line_height(1.5).color(c.muted),
                    checkbox("Show profile wallpapers",self.preferences.wallpapers).on_toggle(Message::Wallpapers).size(18).text_size(13),
                ].spacing(15)));
                let mut shortcuts =
                    column![text("Keyboard shortcuts").size(15).font(BOLD)].spacing(13);
                for (action, key) in [
                    ("App settings", "⌘ ,"),
                    ("Refresh", "⌘ R"),
                    ("Search", "⌘ F"),
                    ("Sign in", "⌘ L"),
                    ("Close app settings / sign-in", "Esc"),
                    ("Quit", "⌘ Q"),
                ] {
                    shortcuts = shortcuts.push(row![
                        text(action).size(12),
                        Space::with_width(Length::Fill),
                        text(key)
                            .font(Font::with_name("Helvetica"))
                            .size(12)
                            .shaping(iced::widget::text::Shaping::Advanced)
                            .color(c.muted)
                    ]);
                }
                content = content.push(card(shortcuts));
                content=content.push(card(column![text("Local storage").size(15).font(BOLD),text(crate::secure_session::description()).size(12).line_height(1.5).color(c.muted),
                    button(text("Reset app preferences").size(12)).on_press(Message::ResetPreferences).padding([9,13]).style(bootstrap_button),
                    text("Resets appearance, reading, and media preferences. Your Ripple account is unaffected.").size(11).color(c.muted),
                ].spacing(13)));
            }
            LocalPage::About => {
                content=content.push(card(column![
                    container(image(self.logo.clone()).width(145).height(63)).padding([18,22]).center_x(Length::Fill).style(|_|container::Style{background:Some(rgb(31,35,41).into()),border:border::rounded(7),..Default::default()}),
                    text("Ripple desktop").size(26).font(BOLD),text(format!("Version {} · {} {}",env!("CARGO_PKG_VERSION"),std::env::consts::OS,std::env::consts::ARCH)).size(13).color(c.muted),
                    text("A native desktop home for Ripple, built with Rust. Connects directly to ripple.tel using the same pages and forms as the website.").size(14).line_height(1.5),
                    row![button(text("Ripple website").size(12).font(BOLD)).on_press(Message::Open("https://ripple.tel/".into())).padding([9,15]).style(primary),button(text("Source license").size(12)).on_press(Message::Local(LocalPage::License)).padding([9,15]).style(bootstrap_button)].spacing(10),
                ].spacing(15)));
                content=content.push(card(column![text("Connection").size(15).font(BOLD),text(if self.snapshot{"Ripple is currently unavailable. You're browsing a saved public preview."}else if self.page.is_some(){"Connected directly to ripple.tel."}else{"Not connected yet."}).size(12).line_height(1.5).color(c.muted),button(text("Check connection").size(12)).on_press(Message::Refresh).padding([8,12]).style(bootstrap_button)].spacing(13)));
                let mut credits =
                    column![text("Built with open source").size(15).font(BOLD)].spacing(11);
                for (name, url) in [
                    ("Rust", "https://www.rust-lang.org/"),
                    ("Iced — desktop interface", "https://iced.rs/"),
                    (
                        "Catppuccin palettes",
                        "https://github.com/catppuccin/palette",
                    ),
                    (
                        "KDE Breeze palettes",
                        "https://github.com/KDE/breeze/tree/master/colors",
                    ),
                    ("Nord palette", "https://github.com/nordtheme/nord"),
                    (
                        "Rosé Pine palettes",
                        "https://github.com/rose-pine/rose-pine-palette",
                    ),
                    ("reqwest — HTTP and sessions", "https://docs.rs/reqwest/"),
                    ("scraper — HTML parsing", "https://docs.rs/scraper/"),
                    ("image — photo decoding", "https://docs.rs/image/"),
                ] {
                    credits = credits.push(
                        button(text(name).size(12).color(c.link))
                            .padding(0)
                            .on_press(Message::Open(url.into()))
                            .style(quiet),
                    );
                }
                content=content.push(card(credits)).push(text("An independent prototype for the Ripple owner's review. Ripple's branding and website content belong to their respective owners.").size(11).line_height(1.5).color(c.muted));
            }
            LocalPage::License => {
                content = content
                    .push(section(
                        "Source license",
                        "Application source: GNU GPL version 2 or later. This desktop binary is distributed under GPL version 3 or later for dependency compatibility.",
                        c,
                    ))
                    .push(card(
                        text(include_str!("../../../LICENSE"))
                            .size(12)
                            .line_height(1.6),
                    ))
                    .push(
                        button(text("< Back to About").size(12))
                            .on_press(Message::Local(LocalPage::About))
                            .padding(8)
                            .style(quiet),
                    );
            }
        }
        content = content.push(
            text(
                if matches!(page, LocalPage::Appearance | LocalPage::General) {
                    "Changes save automatically on this device."
                } else {
                    ""
                },
            )
            .size(11)
            .color(c.muted),
        );
        let main: Element<'_, Message> = if self.width >= 900.0 {
            row![container(navigation).width(175).padding([8, 0]), content]
                .spacing(24)
                .into()
        } else {
            column![
                row![
                    button("Appearance")
                        .on_press(Message::Local(LocalPage::Appearance))
                        .padding(9)
                        .style(bootstrap_button),
                    button("General")
                        .on_press(Message::Local(LocalPage::General))
                        .padding(9)
                        .style(bootstrap_button),
                    button("About")
                        .on_press(Message::Local(LocalPage::About))
                        .padding(9)
                        .style(bootstrap_button)
                ]
                .spacing(8),
                content
            ]
            .spacing(18)
            .into()
        };
        scrollable(
            container(container(main).max_width(990).width(Length::Fill))
                .center_x(Length::Fill)
                .padding([25, 25]),
        )
        .height(Length::Fill)
        .into()
    }
}
fn card<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .padding(21)
        .style(white_panel)
        .into()
}
fn section<'a>(
    title: &'a str,
    description: &'a str,
    c: preferences::Colors,
) -> Element<'a, Message> {
    column![
        text(title).size(25).font(BOLD),
        text(description).size(13).line_height(1.5).color(c.muted)
    ]
    .spacing(8)
    .into()
}
