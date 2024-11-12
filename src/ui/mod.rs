pub fn decorations(ctx: &egui::Context) {
    egui::TopBottomPanel::top("panel")
        .max_height(32.0)
        .frame(
            egui::Frame::none()
                .fill(ctx.style().visuals.window_fill)
                .inner_margin(egui::Margin::ZERO),
        )
        .show_separator_line(false)
        .show(ctx, |ui| {
            let response = ui
                .with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let image = egui::Image::new(egui::include_image!("../../icons/icon.png"))
                        .max_size(egui::vec2(32.0, 32.0))
                        .shrink_to_fit();
                    ui.add(image);

                    ui.label("Stardb-Exporter");

                    ui.add_space(ui.available_width() - 64.0);

                    ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
                    ui.style_mut().visuals.widgets.inactive.weak_bg_fill =
                        ui.visuals().window_fill();

                    let text_format = egui::TextFormat::simple(
                        egui::FontId::new(20.0, egui::FontFamily::Monospace),
                        ui.visuals().text_color(),
                    );

                    let mut minimize_job = egui::text::LayoutJob::default();
                    let mut close_job = egui::text::LayoutJob::default();
                    minimize_job.append("", 0.0, text_format.clone());
                    close_job.append("", 0.0, text_format);

                    let minimize = egui::Button::new(minimize_job)
                        .min_size(egui::vec2(32.0, 32.0))
                        .stroke(egui::Stroke::NONE)
                        .rounding(egui::Rounding::ZERO);

                    let close = egui::Button::new(close_job)
                        .min_size(egui::vec2(32.0, 32.0))
                        .stroke(egui::Stroke::NONE)
                        .rounding(egui::Rounding::ZERO);

                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::BLUE;
                    ui.style_mut().visuals.widgets.active.weak_bg_fill = egui::Color32::BLUE;

                    if ui.add(minimize).clicked() {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    };

                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::RED;
                    ui.style_mut().visuals.widgets.active.weak_bg_fill = egui::Color32::RED;

                    if ui.add(close).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    };
                })
                .response
                .interact(egui::Sense::click_and_drag());

            if response.drag_started_by(egui::PointerButton::Primary) {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        });
}
