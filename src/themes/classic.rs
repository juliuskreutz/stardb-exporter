pub fn colors() -> super::Colors {
    let bg = egui::Color32::from_rgb(13, 9, 36);
    let surface = egui::Color32::from_rgb(24, 22, 70);
    let border = egui::Color32::from_rgb(76, 61, 164);
    let text = egui::Color32::from_rgb(228, 233, 252);
    let accent = egui::Color32::from_rgb(158, 148, 224);

    super::Colors {
        bg,
        surface,
        border,
        text,
        accent,
    }
}
