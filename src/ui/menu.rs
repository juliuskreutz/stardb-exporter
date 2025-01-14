use crate::{
    app::{App, Message},
    games::Game,
};

pub fn show(ui: &mut egui::Ui, app: &App) {
    if ui.button("Honkai: Star Rail").clicked() {
        app.message_tx.send(Message::Game(Game::Hsr)).unwrap();
    }

    if ui.button("Genshin Impact").clicked() {
        app.message_tx.send(Message::Game(Game::Gi)).unwrap();
    }

    if ui.button("Zenless Zone Zero").clicked() {
        app.message_tx.send(Message::Game(Game::Zzz)).unwrap();
    }
}
