use egui_remixicon::icons;

use crate::{
    app::{App, Message, State},
    games,
};

pub fn show(ui: &mut egui::Ui, app: &App) {
    match app.game {
        games::Game::Hsr => {
            if ui.button("Achievement Exporter").clicked() {
                app.message_tx.send(Message::StartRun).unwrap();
            }

            if ui.button("Warp Exporter").clicked() {
                app.message_tx.send(Message::GoTo(State::PullMenu)).unwrap();
            }
        }
        games::Game::Gi => {
            ui.colored_label(ui.visuals().hyperlink_color, format!("{} Make sure, that you fresh started the game before using the achievement exporter!!", icons::INFORMATION_LINE));

            if ui.button("Achievement Exporter").clicked() {
                app.message_tx.send(Message::StartRun).unwrap();
            }

            if ui.button("Wish Exporter").clicked() {
                app.message_tx.send(Message::GoTo(State::PullMenu)).unwrap();
            }
        }
        games::Game::Zzz => {
            ui.colored_label(ui.visuals().hyperlink_color, format!("{} Make sure, that you fresh started the game before using the achievement exporter!!", icons::INFORMATION_LINE));

            if ui.button("Achievement Exporter").clicked() {
                app.message_tx.send(Message::StartRun).unwrap();
            }

            if ui.button("Signal Exporter").clicked() {
                app.message_tx.send(Message::GoTo(State::PullMenu)).unwrap();
            }
        }
    }

    ui.add_space(8.0);

    ui.collapsing("Debug", |ui| {
        if ui.button("Load capture").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Packet capture", &["pcapng"])
                .add_filter("All files", &["*"])
                .pick_file()
        {
            app.message_tx.send(Message::StartReplay(path)).unwrap();
        }
    });
}

/// What was found in a loaded capture.
pub fn capture_report(ui: &mut egui::Ui, report: &str, app: &App) {
    ui.separator();
    ui.label(egui::RichText::new("Capture report").strong());

    if ui.button("Copy report").clicked() {
        if let Err(e) = arboard::Clipboard::new().and_then(|mut c| c.set_text(report)) {
            app.message_tx
                .send(Message::Toast(egui_notify::Toast::error(format!("{e}"))))
                .unwrap();
        } else {
            app.message_tx
                .send(Message::Toast(egui_notify::Toast::success("Copied")))
                .unwrap();
        }
    }

    ui.label(egui::RichText::new(report).monospace());
}
