#![windows_subsystem = "windows"]

use std::path::PathBuf;
use tracing::{info, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;

mod app;
mod games;
mod themes;
mod ui;

fn main() -> anyhow::Result<()> {
    let _guard = tracing_init().expect("Failed to setup logging");

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

fn tracing_init() -> Result<WorkerGuard, Box<dyn std::error::Error>> {

    let mut log_path = PathBuf::from(&std::env::var("APPDATA")?);
    log_path.push("Stardb Exporter");
    log_path.push("log");
    std::fs::create_dir_all(log_path.parent().unwrap())?;

    let file_appender = tracing_appender::rolling::daily(log_path.parent().unwrap(), "log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = fmt()
        .with_max_level(Level::DEBUG)
        .with_writer(non_blocking)
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    info!("Tracing initialized and logging to file.");
    Ok(guard)
}
