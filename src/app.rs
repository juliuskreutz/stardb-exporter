use std::{path::PathBuf, sync::mpsc, thread};

use crate::{
    games::{self, Game},
    themes::{self, Theme},
    ui,
};

pub enum State {
    #[cfg(not(debug_assertions))]
    OutOfDate(self_update::Status),
    Menu,
    Login(String, String),
    Waiting(String),
    PullMenu,
    Game,
    Achievements(Vec<u32>),
    Pulls(String),
    Error(String),
}

pub enum Message {
    GoTo(State),
    Game(Game),
    Theme(Theme),
    #[cfg(not(debug_assertions))]
    Updated(Option<self_update::Status>),
    User(Option<User>),
    Path(PathBuf),
    Logout,
    Toast(egui_notify::Toast),
}

pub struct App {
    pub message_tx: mpsc::Sender<Message>,
    pub message_rx: mpsc::Receiver<Message>,
    pub state: State,
    pub game: games::Game,
    pub toasts: egui_notify::Toasts,
    pub theme: themes::Theme,
    pub user: Option<User>,
    pub paths: Paths,
    pub account_popup_open: bool,
    pub theme_popup_open: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Paths {
    pub zzz: Option<PathBuf>,
    pub hsr: Option<PathBuf>,
    pub gi: Option<PathBuf>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "remixicon".into(),
            egui::FontData::from_static(egui_remixicon::FONT).into(),
        );
        if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            font_keys.push("remixicon".into());
        }

        fonts.font_data.insert(
            "Inter".to_string(),
            egui::FontData::from_static(include_bytes!("../fonts/Inter.ttf")).into(),
        );

        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .push("Inter".to_string());

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

        if let Some(user) = &user {
            let message_tx = message_tx.clone();
            let id = user.id.clone();

            thread::spawn(move || {
                let Some(mut response) = ureq::post("https://stardb.gg/api/users/auth/renew")
                    .header("Cookie", &id)
                    .send_empty()
                    .ok()
                    .and_then(|r| (r.status() == 200).then_some(r))
                else {
                    message_tx
                        .send(Message::GoTo(State::Error(
                            "There was an error renewing your account cookie".to_string(),
                        )))
                        .unwrap();
                    message_tx.send(Message::User(None)).unwrap();
                    return;
                };

                let id = response
                    .headers()
                    .get("Set-Cookie")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .split(';')
                    .next()
                    .unwrap()
                    .to_string();
                let username = response.body_mut().read_json().unwrap();

                let user = User { id, username };
                message_tx.send(Message::User(Some(user))).unwrap();
            });
        }

        Self {
            message_tx,
            message_rx,
            state: State::Waiting("Updating".to_string()),
            game: games::Game::Hsr,
            toasts: egui_notify::Toasts::default().with_anchor(egui_notify::Anchor::BottomRight),
            theme,
            user,
            paths,
            account_popup_open: false,
            theme_popup_open: false,
        }
    }

    fn message(&mut self, message: Message) {
        match message {
            Message::GoTo(state) => {
                self.state = state;
            }
            Message::Game(game) => {
                self.game = game;
                self.state = State::Game;
            }
            #[cfg(not(debug_assertions))]
            Message::Updated(status) => {
                if let Some(status) = status {
                    if status.updated() {
                        self.state = State::OutOfDate(status);

                        let program_name = std::env::args().next().unwrap();
                        let _ = std::process::Command::new(program_name).spawn();
                    } else {
                        self.state = State::Menu;
                    }
                } else {
                    self.state = State::Error("Error updating".to_string());
                }
            }
            Message::Theme(theme) => self.theme = theme,
            Message::User(user) => self.user = user,
            Message::Path(path) => match self.game {
                games::Game::Hsr => self.paths.hsr = Some(path),
                games::Game::Gi => self.paths.gi = Some(path),
                games::Game::Zzz => self.paths.zzz = Some(path),
            },
            Message::Logout => {
                let Some(user) = &self.user else {
                    return;
                };

                let id = user.id.clone();
                self.user = None;

                thread::spawn(move || {
                    let _ = ureq::post("https://stardb.gg/api/users/auth/logout")
                        .header("Cookie", &id)
                        .send_empty();
                });
            }
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui::header::show(ctx, ui, self);

            ui.separator();

            match &self.state {
                State::Waiting(s) => ui::waiting::show(ui, s),
                #[cfg(not(debug_assertions))]
                State::OutOfDate(status) => {
                    ui::waiting::show(
                        ui,
                        &format!("Updated to Version {}. Restarting!", status.version()),
                    );

                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                State::Login(username, password) => ui::login::show(ui, username, password, self),
                State::Menu => ui::menu::show(ui, self),
                State::Achievements(achievements) => ui::achievements::show(ui, achievements, self),
                State::Error(s) => ui::error::show(ui, s),
                State::Game => ui::game::show(ui, self),
                State::Pulls(url) => ui::pulls::show(ui, url, self),
                State::PullMenu => ui::pull_menu::show(ui, self),
            }
        });

        self.toasts.show(ctx);
    }
}

#[cfg(not(debug_assertions))]
fn update(message_tx: &mpsc::Sender<Message>) {
    let message_tx = message_tx.clone();

    let name = if cfg!(all(target_os = "windows", not(feature = "pcap"))) {
        "stardb-exporter-pktmon"
    } else {
        "stardb-exporter-pcap"
    };

    thread::spawn(move || {
        let status = self_update::backends::github::Update::configure()
            .repo_owner("juliuskreutz")
            .repo_name("stardb-exporter")
            .identifier(name)
            .bin_name(name)
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
