pub fn colors() -> super::Colors {
    let bg = egui::Color32::from_rgb(22, 22, 29);
    let surface = egui::Color32::from_rgb(26, 28, 35);
    let text = egui::Color32::from_rgb(225, 233, 239);
    let accent = egui::Color32::from_rgb(205, 141, 226);

    super::Colors {
        bg,
        surface,
        text,
        accent,
    }
}
