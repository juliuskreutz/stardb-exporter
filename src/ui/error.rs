use crate::{
    app::{App, Message, State},
    games::ErrorReport,
    pcapng,
};

pub fn show(ui: &mut egui::Ui, s: &str) {
    ui.label(format!("Error: {s}"));
}

pub fn show_report(ui: &mut egui::Ui, report: &ErrorReport, app: &App) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(egui::RichText::new(&report.title).heading());

        if let Some(detail) = &report.detail {
            ui.label(detail);
        }

        ui.add_space(8.0);

        for (i, step) in report.steps.iter().enumerate() {
            ui.label(format!("{}. {step}", i + 1));
        }

        if report.support {
            ui.add_space(8.0);
            ui.label(format!(
                "If you do not manage to fix the issue on your own, please open a ticket in the discord, give a detailed description of the problem, which game you are trying to read from, and attach the latest log file in %APPDATA%\\Stardb Exporter\\data\\log as well as {}.",
                app.game.capture_file()
            ));
            ui.hyperlink_to("Join the discord", "https://discord.gg/chives");
        }

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if report.capture_report.is_none() {
                if ui.button("Try again").clicked() {
                    app.message_tx.send(Message::StartRun).unwrap();
                }

                if ui.button("Open log folder").clicked()
                    && let Some(dir) = pcapng::log_dir()
                    && let Err(e) = open::that(dir)
                {
                    app.message_tx
                        .send(Message::Toast(egui_notify::Toast::error(format!("{e}"))))
                        .unwrap();
                }
            }

            if ui.button("Back").clicked() {
                app.message_tx.send(Message::GoTo(State::Game)).unwrap();
            }
        });

        if let Some(capture_report) = &report.capture_report {
            ui.add_space(8.0);
            super::game::capture_report(ui, capture_report, app);
        }
    });
}
