pub fn show(ui: &mut egui::Ui, s: &str) {
    ui.horizontal(|ui| {
        ui.label(s);
        ui.add(egui::Spinner::new().color(ui.visuals().text_color()))
    });
}
