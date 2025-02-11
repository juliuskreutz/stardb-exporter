#![windows_subsystem = "windows"]

mod app;
mod games;
mod themes;
mod ui;

fn main() -> anyhow::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_min_inner_size([400.0, 300.0])
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!(
                "../icons/icon.png"
            ))?),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Stardb Exporter",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}
