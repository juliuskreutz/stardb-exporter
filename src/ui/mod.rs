use egui_remixicon::icons;

pub fn decorations(ctx: &egui::Context) {
    egui::TopBottomPanel::top("panel")
        .max_height(32.0)
        .frame(
            egui::Frame::none()
                .fill(ctx.style().visuals.window_fill())
                .inner_margin(egui::Margin::ZERO),
        )
        .show_separator_line(false)
        .show(ctx, |ui| {
            let response = ui
                .horizontal_centered(|ui| {
                    ui.add_space(8.0);

                    let image = egui::Image::new(egui::include_image!("../../icons/icon.svg"))
                        .tint(ui.visuals().text_color())
                        .max_size(egui::vec2(24.0, 24.0));
                    ui.add(image);

                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("Stardb Exporter")
                                .color(ui.visuals().text_color())
                                .font(egui::FontId::proportional(16.0))
                                .strong(),
                        )
                        .selectable(false),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
                        ui.style_mut().spacing.button_padding = egui::vec2(0.0, 0.0);

                        ui.style_mut().visuals.widgets.inactive.weak_bg_fill =
                            ui.visuals().window_fill();

                        let mut text_format = egui::TextFormat::simple(
                            egui::FontId::proportional(18.0),
                            ui.visuals().text_color(),
                        );
                        text_format.valign = egui::Align::Center;

                        let mut minimize_job = egui::text::LayoutJob::single_section(
                            icons::SUBTRACT_FILL.to_string(),
                            text_format.clone(),
                        );
                        minimize_job.first_row_min_height = 32.0;

                        let mut close_job = egui::text::LayoutJob::single_section(
                            icons::CLOSE_FILL.to_string(),
                            text_format.clone(),
                        );
                        close_job.first_row_min_height = 32.0;

                        let minimize = egui::Button::new(minimize_job)
                            .min_size(egui::vec2(32.0, 32.0))
                            .stroke(egui::Stroke::NONE)
                            .rounding(egui::Rounding::ZERO);

                        let close = egui::Button::new(close_job)
                            .min_size(egui::vec2(32.0, 32.0))
                            .stroke(egui::Stroke::NONE)
                            .rounding(egui::Rounding::ZERO);

                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::RED;
                        ui.style_mut().visuals.widgets.active.weak_bg_fill = egui::Color32::RED;

                        if ui.add(close).clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        };

                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::BLUE;
                        ui.style_mut().visuals.widgets.active.weak_bg_fill = egui::Color32::BLUE;

                        if ui.add(minimize).clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        };
                    });
                })
                .response
                .interact(egui::Sense::click_and_drag());

            if response.drag_started_by(egui::PointerButton::Primary) {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        });
}
