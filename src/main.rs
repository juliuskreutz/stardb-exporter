#![windows_subsystem = "windows"]

use std::path::PathBuf;

mod app;
mod games;
mod themes;
mod ui;

fn main() -> anyhow::Result<()> {
    tracing_init()?;

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

fn tracing_init() -> anyhow::Result<()> {
    let mut log_path = PathBuf::from(&std::env::var("APPDATA")?);
    log_path.push("Stardb Exporter");
    log_path.push("log");

    let appender = tracing_appender::rolling::daily(log_path, "log");
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking_appender)
        .with_ansi(false)
        .init();
    tracing::info!("Tracing initialized and logging to file.");

    Ok(())
}
