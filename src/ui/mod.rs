use egui::Widget;

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
            let max_rect = ui.max_rect();
            let rect = max_rect.with_max_y(32.0);

            /*let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

            if response.drag_started_by(egui::PointerButton::Primary) {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }*/

            let response = ui
                .with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label("Hi");

                    ui.add_space(ui.available_width() - 64.0);

                    ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

                    let mut job = egui::text::LayoutJob::default();
                    job.append(
                        "",
                        0.0,
                        egui::TextFormat::simple(
                            egui::FontId::new(20.0, egui::FontFamily::Monospace),
                            ui.visuals().text_color(),
                        ),
                    );

                    let minimize = egui::Button::new(job)
                        .min_size(egui::vec2(32.0, 32.0))
                        .stroke(egui::Stroke::NONE)
                        .rounding(egui::Rounding::ZERO);

                    let close = egui::Button::new("")
                        .min_size(egui::vec2(32.0, 32.0))
                        .stroke(egui::Stroke::NONE)
                        .rounding(egui::Rounding::ZERO);

                    if ui.add(minimize).clicked() {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    };

                    if ui.add(close).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    };
                })
                .response
                .interact(egui::Sense::click_and_drag());

            if response.drag_started_by(egui::PointerButton::Primary) {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            /*let response = ui
            .with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label("Hi");

                ui.expand_to_include_rect(rect);

                ui.label("Hi");
            })
            .response
            .interact(egui::Sense::click_and_drag());*/

            /* ui.painter().text(
                rect.left_center(),
                egui::Align2::LEFT_CENTER,
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

            let minimize_response =
                ui.interact(rect, egui::Id::new("minimize"), egui::Sense::click());

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
            ); */
        });
}
