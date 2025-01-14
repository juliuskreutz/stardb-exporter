use egui_remixicon::icons;

use crate::{
    app::{App, Message, State},
    games,
};

pub fn show(ui: &mut egui::Ui, app: &App) {
    match app.game {
        games::Game::Hsr => {
            if ui.button("Achievement Exporter").clicked() {
                app.game.achievements(&app.message_tx);
                app.message_tx
                    .send(Message::GoTo(State::Waiting("Preparing".to_string())))
                    .unwrap();
            }

            if ui.button("Warp Exporter").clicked() {
                app.message_tx.send(Message::GoTo(State::PullMenu)).unwrap();
            }
        }
        games::Game::Gi => {
            ui.colored_label(ui.visuals().hyperlink_color, format!("{} Make sure, that you fresh started the game before using the achievement exporter!!", icons::INFORMATION_LINE));

            if ui.button("Achievement Exporter").clicked() {
                app.game.achievements(&app.message_tx);
                app.message_tx
                    .send(Message::GoTo(State::Waiting("Preparing".to_string())))
                    .unwrap();
            }

            if ui.button("Wish Exporter").clicked() {
                app.message_tx.send(Message::GoTo(State::PullMenu)).unwrap();
            }
        }
        games::Game::Zzz => {
            if ui.button("Signal Exporter").clicked() {
                app.message_tx.send(Message::GoTo(State::PullMenu)).unwrap();
            }
        }
    }
}
