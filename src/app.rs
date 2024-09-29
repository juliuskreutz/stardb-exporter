use std::{sync::mpsc, thread};

use crate::games;

#[derive(Default)]
enum State {
    #[default]
    Init,
    Updating(mpsc::Receiver<Option<self_update::Status>>),
    OutOfDate(self_update::Status),
    Ready,
    Running(mpsc::Receiver<Option<String>>),
    Finished,
}

#[derive(Default)]
pub struct App {
    state: State,
    game: games::Game,
    messages: Vec<String>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let game = match self.game {
                games::Game::Hsr => "Honkai: Star Rail",
                games::Game::Gi => "Genshin Impact",
            };

            ui.heading(format!("Stardb Exporter - {game}"));

            match &self.state {
                State::Init => self.state = State::Updating(update()),
                State::Updating(update_rx) => {
                    ui.horizontal(|ui| {
                        ui.label("Updating");
                        ui.spinner()
                    });

                    if let Ok(status) = update_rx.try_recv() {
                        let mut updated = false;

                        if let Some(status) = status {
                            updated = status.updated();

                            if updated {
                                self.state = State::OutOfDate(status);
                            }
                        }

                        if !updated {
                            self.state = State::Ready;
                        }
                    }
                }
                State::OutOfDate(status) => {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "Updated to Version {}. Please restart program",
                            status.version()
                        ))
                    });
                }
                State::Ready => {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.game, games::Game::Hsr, "Honkai: Star Rail");
                        ui.selectable_value(&mut self.game, games::Game::Gi, "Genshin Impact");

                        if ui.button("▶").clicked() {
                            self.state = State::Running(self.game.run());
                        }
                    });
                }
                State::Running(log_rx) => {
                    ui.horizontal(|ui| {
                        ui.label("Running");
                        ui.spinner();
                    });

                    egui::containers::ScrollArea::vertical().show(ui, |ui| {
                        for message in &self.messages {
                            ui.label(message);
                        }
                    });

                    match log_rx.try_recv() {
                        Ok(None) => self.state = State::Finished,
                        Ok(Some(s)) => self.messages.push(s),
                        _ => {}
                    }
                }
                State::Finished => {
                    ui.label("Finished");

                    egui::containers::ScrollArea::vertical().show(ui, |ui| {
                        for message in &self.messages {
                            ui.label(message);
                        }
                    });
                }
            }
        });
    }
}

fn update() -> mpsc::Receiver<Option<self_update::Status>> {
    let (update_tx, update_rx) = mpsc::channel();

    thread::spawn(move || -> anyhow::Result<()> {
        update_tx.send(
            self_update::backends::github::Update::configure()
                .repo_owner("juliuskreutz")
                .repo_name("stardb-exporter")
                .bin_name("stardb-exporter")
                .current_version(self_update::cargo_crate_version!())
                .no_confirm(true)
                .build()
                .ok()
                .and_then(|e| e.update().ok()),
        )?;

        Ok(())
    });

    update_rx
}
