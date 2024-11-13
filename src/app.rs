use std::{path::PathBuf, sync::mpsc, thread};

use crate::{games, themes, ui};

pub enum State {
    #[cfg(not(debug_assertions))]
    OutOfDate(self_update::Status),
    Menu,
    Login,
    Waiting(String),
    PullMenu,
    Game,
    Achievements(Vec<u32>),
    Pulls(String),
    Error(String),
}

pub enum Message {
    GoTo(State),
    #[cfg(not(debug_assertions))]
    Updated(Option<self_update::Status>),
    LoggedIn(User),
    Error(String),
    Toast(egui_notify::Toast),
    Achievements(Vec<u32>),
}

pub struct App {
    message_tx: mpsc::Sender<Message>,
    message_rx: mpsc::Receiver<Message>,
    state: State,
    game: games::Game,
    username: String,
    password: String,
    toasts: egui_notify::Toasts,
    theme: themes::Theme,
    user: Option<User>,
    paths: Paths,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct User {
    id: String,
    username: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Paths {
    zzz: Option<PathBuf>,
    hsr: Option<PathBuf>,
    gi: Option<PathBuf>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "JetBrainsMonoNerdFont".to_string(),
            egui::FontData::from_static(include_bytes!(
                "../fonts/JetBrainsMonoNerdFont-Regular.ttf"
            )),
        );

        fonts.font_data.insert(
            "JetBrainsMonoNerdFontMono".to_string(),
            egui::FontData::from_static(include_bytes!(
                "../fonts/JetBrainsMonoNerdFontMono-Regular.ttf"
            )),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "JetBrainsMonoNerdFont".to_string());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "JetBrainsMonoNerdFontMono".to_string());

        cc.egui_ctx.set_fonts(fonts);

        let theme: themes::Theme = cc
            .storage
            .and_then(|s| eframe::get_value(s, "theme"))
            .unwrap_or_default();

        let user: Option<User> = cc
            .storage
            .and_then(|s| eframe::get_value(s, "user").unwrap_or_default());

        let paths: Paths = cc
            .storage
            .and_then(|s| eframe::get_value(s, "paths"))
            .unwrap_or_default();

        cc.egui_ctx.set_style(theme.style());

        let (message_tx, message_rx) = mpsc::channel();

        update(&message_tx);

        Self {
            message_tx,
            message_rx,
            state: State::Waiting("Updating".to_string()),
            game: games::Game::Hsr,
            username: String::new(),
            password: String::new(),
            toasts: egui_notify::Toasts::default().with_anchor(egui_notify::Anchor::BottomRight),
            theme,
            user,
            paths,
        }
    }

    fn message(&mut self, message: Message) {
        match message {
            Message::GoTo(state) => self.state = state,
            #[cfg(not(debug_assertions))]
            Message::Updated(status) => {
                if let Some(status) = status {
                    if status.updated() {
                        self.state = State::OutOfDate(status);

                        let program_name = std::env::args().next().unwrap();
                        std::process::Command::new(program_name).spawn().unwrap();
                    } else {
                        self.state = State::Menu;
                    }
                } else {
                    self.state = State::Error("Error updating".to_string());
                }
            }
            Message::LoggedIn(user) => {
                self.user = Some(user);
                self.state = State::Menu;
            }
            Message::Error(e) => self.state = State::Error(e),
            Message::Achievements(vec) => self.state = State::Achievements(vec),
            Message::Toast(toast) => {
                self.toasts.add(toast);
            }
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "user", &self.user);
        eframe::set_value(storage, "theme", &self.theme);
        eframe::set_value(storage, "paths", &self.paths);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(message) = self.message_rx.try_recv() {
            self.message(message);
        }

        ctx.set_style(self.theme.style());

        ui::decorations(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.set_height(48.0);

                ui.add_space(32.0);

                let waiting = matches!(self.state, State::Waiting(_));

                let heading = ui.add_enabled(
                    !waiting,
                    egui::Label::new(egui::RichText::new("󱞴 Menu").heading()),
                );

                if heading.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if heading.clicked() {
                    self.message_tx.send(Message::GoTo(State::Menu)).unwrap();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(32.0);

                    let login_text = self
                        .user
                        .as_ref()
                        .map(|u| u.username.as_str())
                        .unwrap_or("Login");

                    let login_button = ui.add_enabled(!waiting, egui::Button::new(login_text));
                    if login_button.clicked() {
                        self.message_tx.send(Message::GoTo(State::Login)).unwrap();
                    }

                    let height = login_button.rect.height();

                    ui.style_mut().spacing.button_padding = egui::vec2(0.0, 0.0);
                    let button = egui::Button::new(
                        egui::RichText::new("").font(egui::FontId::monospace(24.0)),
                    );
                    let button = button.min_size(egui::vec2(48.0, height));

                    let color_button = ui.add(button);
                    let popup_id = color_button.id.with("popup");

                    let is_popup_open = ui.memory(|m| m.is_popup_open(popup_id));

                    if is_popup_open {
                        egui::popup::popup_above_or_below_widget(
                            ui,
                            popup_id,
                            &color_button,
                            egui::AboveOrBelow::Below,
                            egui::PopupCloseBehavior::CloseOnClick,
                            |ui| {
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                ui.visuals_mut().widgets.inactive.bg_stroke.color =
                                    ui.visuals().widgets.active.bg_stroke.color;

                                ui.selectable_value(&mut self.theme, themes::Theme::Dark, "Dark");
                                ui.selectable_value(&mut self.theme, themes::Theme::Light, "Light");
                                ui.selectable_value(
                                    &mut self.theme,
                                    themes::Theme::Classic,
                                    "Classic",
                                );
                            },
                        );
                    }

                    if color_button.clicked() {
                        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                    }
                });
            });

            ui.separator();

            match &self.state {
                State::Waiting(s) => {
                    ui.horizontal(|ui| {
                        ui.label(s);
                        ui.add(egui::Spinner::new().color(ui.visuals().text_color()))
                    });
                }
                #[cfg(not(debug_assertions))]
                State::OutOfDate(status) => {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "Updated to Version {}. Restarting!",
                            status.version()
                        ))
                    });

                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                State::Login => {
                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.username);

                    ui.label("Password:");
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));

                    if ui.button("Login").clicked() {
                        login(&self.username, &self.password, &self.message_tx);

                        self.username.clear();
                        self.password.clear();

                        self.message_tx
                            .send(Message::GoTo(State::Waiting("Loggin In".to_string())))
                            .unwrap();
                    }
                }
                State::Menu => {
                    if ui.button("Honkai: Star Rail").clicked() {
                        self.game = games::Game::Hsr;
                        self.state = State::Game;
                    }

                    if ui.button("Genshin Impact").clicked() {
                        self.game = games::Game::Gi;
                        self.state = State::Game;
                    }

                    if ui.button("Zenless Zone Zero").clicked() {
                        self.game = games::Game::Zzz;
                        self.state = State::Game;
                    }
                }
                State::Achievements(achievements) => {
                    let key = match self.game {
                        games::Game::Hsr => {
                            ui.heading("HSR");
                            "hsr_achievements"
                        }
                        games::Game::Gi => {
                            ui.heading("GI");
                            "gi_achievements"
                        }
                        _ => unimplemented!(),
                    };

                    ui.label("Finished");

                    if ui
                        .button(format!(
                            "Copy {} achievements to clipboard",
                            achievements.len()
                        ))
                        .clicked()
                    {
                        if let Err(e) = arboard::Clipboard::new().and_then(|mut c| {
                            c.set_text(serde_json::json!({ key: achievements }).to_string())
                        }) {
                            self.message_tx.send(Message::Error(e.to_string())).unwrap();
                        } else {
                            self.toasts.success("Copied");
                        }
                    }

                    ui.hyperlink_to("Click here to import", "https://stardb.gg/import");

                    if let Some(user) = &self.user {
                        if ui
                            .button(format!("Sync to account: \"{}\"", user.username))
                            .clicked()
                        {
                            self.toasts.info("Syncing");

                            let prefix = match self.game {
                                games::Game::Hsr => "",
                                games::Game::Gi => "gi/",
                                _ => unimplemented!(),
                            };

                            let url = format!(
                                "https://stardb.gg/api/users/me/{prefix}achievements/completed"
                            );

                            {
                                let message_tx = self.message_tx.clone();
                                let id = user.id.clone();
                                let achievements = achievements.clone();

                                thread::spawn(move || {
                                    match ureq::put(&url).set("Cookie", &id).send_json(achievements)
                                    {
                                        Ok(r) => {
                                            if r.status() == 200 {
                                                message_tx
                                                    .send(Message::Toast(
                                                        egui_notify::Toast::success("Synced"),
                                                    ))
                                                    .unwrap();
                                            } else {
                                                message_tx
                                                    .send(Message::Toast(
                                                        egui_notify::Toast::error(
                                                            "Error. Try Relogging",
                                                        ),
                                                    ))
                                                    .unwrap();
                                            }
                                        }
                                        Err(e) => {
                                            message_tx.send(Message::Error(e.to_string())).unwrap();
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
                State::Error(e) => {
                    ui.label(format!("Error: {e}"));
                }
                State::Game => match self.game {
                    games::Game::Hsr => {
                        ui.heading("HSR");

                        if ui.button("Achievement Exporter").clicked() {
                            self.game.achievements(&self.message_tx);
                            self.state = State::Waiting("Preparing".to_string());
                        }

                        if ui.button("Warp Exporter").clicked() {
                            self.state = State::PullMenu;
                        }
                    }
                    games::Game::Gi => {
                        ui.heading("GI");

                        if ui.button("Achievement Exporter").clicked() {
                            self.game.achievements(&self.message_tx);
                            self.state = State::Waiting("Preparing".to_string());
                        }

                        if ui.button("Wish Exporter").clicked() {
                            self.state = State::PullMenu;
                        }
                    }
                    games::Game::Zzz => {
                        ui.heading("ZZZ");

                        if ui.button("Signal Exporter").clicked() {
                            self.state = State::PullMenu;
                        }
                    }
                },
                State::Pulls(url) => {
                    ui.label("Finished");

                    if ui.button("Copy url to clipboard").clicked() {
                        if let Err(e) =
                            arboard::Clipboard::new().and_then(|mut c| c.set_text(url.clone()))
                        {
                            self.message_tx.send(Message::Error(e.to_string())).unwrap();
                        } else {
                            self.toasts.success("Copied");
                        }
                    }

                    let import_url = match self.game {
                        games::Game::Hsr => "https://stardb.gg/warp-import",
                        games::Game::Gi => "https://stardb.gg/genshin/wish-import",
                        games::Game::Zzz => "https://stardb.gg/zzz/signal-import",
                    };

                    ui.hyperlink_to("Click here to import", import_url);

                    if ui.button("Sync to stardb").clicked() {
                        let import_url = match self.game {
                            games::Game::Hsr => "https://stardb.gg/api/warps-import",
                            games::Game::Gi => "https://stardb.gg/api/gi/wishes-import",
                            games::Game::Zzz => "https://stardb.gg/api/zzz/signals-import",
                        };

                        let request = if let Some(user) = &self.user {
                            ureq::post(import_url).set("Cookie", &user.id)
                        } else {
                            ureq::post(import_url)
                        };

                        match request.send_json(serde_json::json!({"url": url})) {
                            Ok(r) => {
                                self.toasts.success(format!(
                                    "Synced uid {}",
                                    r.into_json::<serde_json::Value>().unwrap()["uid"]
                                ));
                            }
                            Err(e) => {
                                self.toasts.error(format!("Error: {e}"));
                            }
                        }
                    }
                }
                State::PullMenu => {
                    match self.game {
                        games::Game::Hsr => {
                            ui.heading("HSR - Warp Exporter");
                            ui.label(format!(
                                "Path: {}",
                                self.paths
                                    .hsr
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or("None".to_string())
                            ));
                        }
                        games::Game::Gi => {
                            ui.heading("GI - Wish Exporter");
                            ui.label(format!(
                                "Path: {}",
                                self.paths
                                    .gi
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or("None".to_string())
                            ));
                        }
                        games::Game::Zzz => {
                            ui.heading("ZZZ - Signal Exporter");
                            ui.label(format!(
                                "Path: {}",
                                self.paths
                                    .zzz
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or("None".to_string())
                            ));
                        }
                    }

                    if ui.button("Automatic").clicked() {
                        match self.game.game_path() {
                            Ok(path) => match self.game {
                                games::Game::Hsr => self.paths.hsr = Some(path),
                                games::Game::Gi => self.paths.gi = Some(path),
                                games::Game::Zzz => self.paths.zzz = Some(path),
                            },
                            Err(e) => self.message_tx.send(Message::Error(e.to_string())).unwrap(),
                        }
                    }

                    if ui
                        .button("Manual selection (e.g. D:\\Star Rail\\Games\\StarRail_Data)")
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            match self.game {
                                games::Game::Hsr => self.paths.hsr = Some(path),
                                games::Game::Gi => self.paths.gi = Some(path),
                                games::Game::Zzz => self.paths.zzz = Some(path),
                            }
                        }
                    }

                    if let Some(path) = match self.game {
                        games::Game::Hsr => &self.paths.hsr,
                        games::Game::Gi => &self.paths.gi,
                        games::Game::Zzz => &self.paths.zzz,
                    } {
                        if ui.button("Get Url").clicked() {
                            let message_tx = self.message_tx.clone();
                            let path = path.clone();

                            thread::spawn(move || {
                                match games::pulls_from_game_path(&path) {
                                    Ok(url) => message_tx.send(Message::GoTo(State::Pulls(url))),
                                    Err(e) => {
                                        message_tx.send(Message::GoTo(State::Error(e.to_string())))
                                    }
                                }
                                .unwrap()
                            });

                            self.state = State::Waiting("Running".to_string());
                        }
                    } else {
                        ui.add_enabled(false, egui::Button::new("Get Url"));
                    }
                }
            }
        });

        self.toasts.show(ctx);
    }
}

fn login(username: &str, password: &str, message_tx: &mpsc::Sender<Message>) {
    let username = username.to_string();
    let password = password.to_string();
    let message_tx = message_tx.clone();

    thread::spawn(move || {
        let json = serde_json::json!({
            "username": username,
            "password": password
        });

        let id = ureq::post("https://stardb.gg/api/users/auth/login")
            .send_json(json)
            .ok()
            .and_then(|r| {
                r.header("Set-Cookie")
                    .and_then(|id| id.split(';').next())
                    .map(|s| s.to_string())
            });

        if let Some(id) = id {
            let username = username.to_string();

            let user = User { id, username };

            message_tx.send(Message::LoggedIn(user)).unwrap();
        } else {
            message_tx
                .send(Message::Error(
                    "There was an error during the login".to_string(),
                ))
                .unwrap();
        }
    });
}

#[cfg(not(debug_assertions))]
fn update(message_tx: &mpsc::Sender<Message>) {
    let message_tx = message_tx.clone();

    thread::spawn(move || {
        let status = self_update::backends::github::Update::configure()
            .repo_owner("juliuskreutz")
            .repo_name("stardb-exporter")
            .bin_name("stardb-exporter")
            .current_version(self_update::cargo_crate_version!())
            .no_confirm(true)
            .build()
            .ok()
            .and_then(|e| e.update().ok());

        message_tx.send(Message::Updated(status)).unwrap();
    });
}

#[cfg(debug_assertions)]
fn update(message_tx: &mpsc::Sender<Message>) {
    message_tx.send(Message::GoTo(State::Menu)).unwrap();
}
