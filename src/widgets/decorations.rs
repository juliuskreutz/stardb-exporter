pub struct Decorations;

impl egui::Widget for Decorations {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let max_rect = ui.max_rect();
        let rect = max_rect.with_max_y(32.0);

        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

        if response.drag_started_by(egui::PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Stardb Exporter",
            egui::FontId::new(20.0, egui::FontFamily::Proportional),
            ui.visuals().text_color(),
        );

        let rect = rect.with_min_x(rect.max.x - 32.0);

        let close_response = ui.interact(rect, egui::Id::new("close"), egui::Sense::click());

        if close_response.clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if close_response.hovered() {
            ui.painter()
                .rect_filled(rect, egui::Rounding::ZERO, egui::Color32::RED);
        }

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "",
            egui::FontId::new(20.0, egui::FontFamily::Monospace),
            ui.visuals().text_color(),
        );

        let rect = rect.with_max_x(rect.max.x - 32.0);
        let rect = rect.with_min_x(rect.max.x - 32.0);

        let minimize_response = ui.interact(rect, egui::Id::new("minimize"), egui::Sense::click());

        if minimize_response.clicked() {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        let text_color = if minimize_response.hovered() {
            ui.painter().rect_filled(
                rect,
                egui::Rounding::ZERO,
                ui.visuals().widgets.hovered.weak_bg_fill,
            );

            ui.visuals().widgets.hovered.fg_stroke.color
        } else {
            ui.visuals().text_color()
        };

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "",
            egui::FontId::new(20.0, egui::FontFamily::Monospace),
            text_color,
        );

        response
    }
}
