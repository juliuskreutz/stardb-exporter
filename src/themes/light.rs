pub fn colors() -> super::Colors {
    let bg = egui::Color32::from_rgb(251, 229, 223);
    let surface = egui::Color32::from_rgb(252, 243, 237);
    let text = egui::Color32::from_rgb(101, 29, 27);
    let accent = egui::Color32::from_rgb(254, 139, 143);

    super::Colors {
        bg,
        surface,
        text,
        accent,
    }
}
