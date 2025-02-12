use egui_remixicon::icons;

use crate::{
    app::{App, Message, State},
    games, themes,
};

pub fn show(ctx: &egui::Context, ui: &mut egui::Ui, app: &App) {
    ui.horizontal(|ui| {
        ui.set_height(36.0);

        ui.add_space(32.0);

        let waiting = matches!(app.state, State::Waiting(_));

        let heading_text = match app.state {
            State::Game | State::Achievements(_) | State::PullMenu => match app.game {
                games::Game::Hsr => "Honkai Star Rail",
                games::Game::Gi => "Genshin Impact",
                games::Game::Zzz => "Zenless Zone Zero",
            },
            _ => "Menu",
        };

        let heading = ui.add_enabled(
            !waiting,
            egui::Label::new(
                egui::RichText::new(format!("{} {heading_text}", icons::ARROW_LEFT_UP_LINE))
                    .heading(),
            ),
        );

        if heading.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if heading.clicked() {
            app.message_tx.send(Message::GoTo(State::Menu)).unwrap();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(32.0);

            let height = if let Some(user) = &app.user {
                let mut icon_format = egui::TextFormat::simple(
                    egui::FontId::proportional(20.0),
                    ui.visuals().text_color(),
                );
                icon_format.valign = egui::Align::Center;

                let mut text_format = egui::TextFormat::simple(
                    egui::FontId::proportional(14.0),
                    ui.visuals().text_color(),
                );
                text_format.valign = egui::Align::Center;

                let mut username_job = egui::text::LayoutJob::default();
                username_job.append(icons::ACCOUNT_CIRCLE_LINE, 0.0, icon_format.clone());
                username_job.append(&user.username, 8.0, text_format.clone());

                let account_button = ui.add_enabled(!waiting, egui::Button::new(username_job));
                let account_popup_id = account_button.id.with("popup");

                let is_account_popup_open = ui.memory(|m| m.is_popup_open(account_popup_id));

                if is_account_popup_open {
                    egui::popup::popup_above_or_below_widget(
                        ui,
                        account_popup_id,
                        &account_button,
                        egui::AboveOrBelow::Below,
                        egui::PopupCloseBehavior::CloseOnClick,
                        |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                            ui.visuals_mut().widgets.inactive.bg_stroke.color =
                                ui.visuals().widgets.active.bg_stroke.color;

                            let mut icon_format = egui::TextFormat::simple(
                                egui::FontId::proportional(20.0),
                                ui.visuals().text_color(),
                            );
                            icon_format.valign = egui::Align::Center;

                            let mut text_format = egui::TextFormat::simple(
                                egui::FontId::proportional(14.0),
                                ui.visuals().text_color(),
                            );
                            text_format.valign = egui::Align::Center;

                            let mut website_job = egui::text::LayoutJob::default();
                            website_job.append(icons::LINK, 0.0, icon_format.clone());
                            website_job.append("Website", 8.0, text_format.clone());

                            let mut logout_job = egui::text::LayoutJob::default();
                            logout_job.append(icons::LOGOUT_BOX_LINE, 0.0, icon_format.clone());
                            logout_job.append("Logout", 8.0, text_format.clone());

                            if ui.button(website_job).clicked() {
                                let url = match app.state {
                                    State::Achievements(_) => app.game.achievement_url(),
                                    State::PullMenu | State::Pulls(_) => app.game.pull_url(),
                                    _ => "https://stardb.gg".to_string(),
                                };

                                if let Err(e) = open::that(url) {
                                    app.message_tx
                                        .send(Message::Toast(egui_notify::Toast::error(format!(
                                            "{e}"
                                        ))))
                                        .unwrap();
                                }
                            }

                            if ui.button(logout_job).clicked() {
                                app.message_tx.send(Message::Logout).unwrap();
                            }
                        },
                    );
                }

                if account_button.clicked() {
                    ui.memory_mut(|mem| mem.toggle_popup(account_popup_id));
                }

                account_button.rect.height()
            } else {
                ui.scope(|ui| {
                    let text = ui.visuals().widgets.inactive.weak_bg_fill;
                    let accent = ui.visuals().hyperlink_color;
                    let accent_hover = ui.visuals().hyperlink_color.gamma_multiply(0.8);
                    ui.visuals_mut().widgets.inactive.fg_stroke.color = text;
                    ui.visuals_mut().widgets.inactive.weak_bg_fill = accent;
                    ui.visuals_mut().widgets.inactive.bg_stroke.color = accent;
                    ui.visuals_mut().widgets.hovered.fg_stroke.color = text;
                    ui.visuals_mut().widgets.hovered.weak_bg_fill = accent_hover;
                    ui.visuals_mut().widgets.hovered.bg_stroke.color = accent_hover;
                    ui.visuals_mut().widgets.active.fg_stroke.color = text;
                    ui.visuals_mut().widgets.active.weak_bg_fill = accent_hover;
                    ui.visuals_mut().widgets.active.bg_stroke.color = accent_hover;

                    let mut icon_format =
                        egui::TextFormat::simple(egui::FontId::proportional(20.0), text);
                    icon_format.valign = egui::Align::Center;

                    let mut text_format =
                        egui::TextFormat::simple(egui::FontId::proportional(14.0), text);
                    text_format.valign = egui::Align::Center;

                    let mut login_job = egui::text::LayoutJob::default();
                    login_job.append(icons::LOGIN_BOX_LINE, 0.0, icon_format.clone());
                    login_job.append("Login", 8.0, text_format.clone());

                    let login_button = ui.add_enabled(!waiting, egui::Button::new(login_job));
                    if login_button.clicked() {
                        app.message_tx
                            .send(Message::GoTo(State::Login(String::new(), String::new())))
                            .unwrap();
                    }

                    login_button.rect.height()
                })
                .inner
            };

            let old_button_padding = ui.style().spacing.button_padding;
            ui.style_mut().spacing.button_padding = egui::vec2(0.0, 0.0);
            let button = egui::Button::new(
                egui::RichText::new(egui_remixicon::icons::PALETTE_LINE).size(20.0),
            );
            let button = button.min_size(egui::vec2(48.0, height));

            let color_button = ui.add(button);
            let color_popup_id = color_button.id.with("popup");

            let is_color_popup_open = ui.memory(|m| m.is_popup_open(color_popup_id));

            if is_color_popup_open {
                egui::popup::popup_above_or_below_widget(
                    ui,
                    color_popup_id,
                    &color_button,
                    egui::AboveOrBelow::Below,
                    egui::PopupCloseBehavior::CloseOnClick,
                    |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.visuals_mut().widgets.inactive.bg_stroke.color =
                            ui.visuals().widgets.active.bg_stroke.color;

                        let mut icon_format = egui::TextFormat::simple(
                            egui::FontId::proportional(20.0),
                            ui.visuals().text_color(),
                        );
                        icon_format.valign = egui::Align::Center;

                        let mut text_format = egui::TextFormat::simple(
                            egui::FontId::proportional(14.0),
                            ui.visuals().text_color(),
                        );
                        text_format.valign = egui::Align::Center;

                        let mut dark_job = egui::text::LayoutJob::default();
                        dark_job.append(icons::MOON_LINE, 0.0, icon_format.clone());
                        dark_job.append("Dark", 8.0, text_format.clone());

                        let mut light_job = egui::text::LayoutJob::default();
                        light_job.append(icons::SUN_LINE, 0.0, icon_format.clone());
                        light_job.append("Light", 8.0, text_format.clone());

                        let mut classic_job = egui::text::LayoutJob::default();
                        classic_job.append(icons::BARD_LINE, 0.0, icon_format.clone());
                        classic_job.append("Classic", 8.0, text_format.clone());

                        let mut theme = app.theme;

                        ui.selectable_value(&mut theme, themes::Theme::Dark, dark_job);
                        ui.selectable_value(&mut theme, themes::Theme::Light, light_job);
                        ui.selectable_value(&mut theme, themes::Theme::Classic, classic_job);

                        app.message_tx.send(Message::Theme(theme)).unwrap();
                    },
                );
            }

            if color_button.clicked() {
                ui.memory_mut(|mem| mem.toggle_popup(color_popup_id));
            }

            ui.style_mut().spacing.button_padding = old_button_padding;
            let text = egui::Color32::BLACK;
            let accent = egui::Color32::from_rgb(250, 204, 21);
            let accent_hover = egui::Color32::from_rgb(253, 224, 71);
            ui.visuals_mut().widgets.inactive.fg_stroke.color = text;
            ui.visuals_mut().widgets.inactive.weak_bg_fill = accent;
            ui.visuals_mut().widgets.inactive.bg_stroke.color = accent;
            ui.visuals_mut().widgets.hovered.fg_stroke.color = text;
            ui.visuals_mut().widgets.hovered.weak_bg_fill = accent_hover;
            ui.visuals_mut().widgets.hovered.bg_stroke.color = accent_hover;
            ui.visuals_mut().widgets.active.fg_stroke.color = text;
            ui.visuals_mut().widgets.active.weak_bg_fill = accent_hover;
            ui.visuals_mut().widgets.active.bg_stroke.color = accent_hover;

            let mut icon_format = egui::TextFormat::simple(
                egui::FontId::proportional(20.0),
                text
            );
            icon_format.valign = egui::Align::Center;

            let mut text_format = egui::TextFormat::simple(
                egui::FontId::proportional(14.0),
                text
            );
            text_format.valign = egui::Align::Center;

            let mut lootbar_job = egui::text::LayoutJob::default();
            lootbar_job.append(icons::LINKS_LINE, 0.0, icon_format.clone());
            lootbar_job.append("Lootbar", 8.0, text_format.clone());

            let button = egui::Button::new(lootbar_job);

            if ui.add(button).clicked() {
                if let Err(e) = open::that("https://lootbar.gg/index?utm_source=Affiliate&utm_medium=Affiliate&utm_campaign=lHBYqExxGc") {
                    app.message_tx
                        .send(Message::Toast(egui_notify::Toast::error(format!("{e}"))))
                        .unwrap();
                }
            }
        });
    });
}
