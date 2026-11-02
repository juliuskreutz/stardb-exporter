use crate::app::{App, Message};

pub fn show(ui: &mut egui::Ui, s: &str, app: &App) {
    ui.horizontal(|ui| {
        ui.label(s);
        ui.add(egui::Spinner::new().color(ui.visuals().text_color()))
    });

    if let Some(warning) = &app.run_warning {
        ui.colored_label(ui.visuals().warn_fg_color, warning);
    }

    if app.run.as_ref().is_some_and(|run| !run.is_stopped()) && ui.button("Stop").clicked() {
        app.message_tx.send(Message::StopRun).unwrap();
    }
}
