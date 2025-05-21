#![windows_subsystem = "windows"]

mod app;
mod games;
mod themes;
mod ui;

const APP_ID: &str = "Stardb Exporter";

fn main() -> anyhow::Result<()> {
    let _guard = tracing_init()?;

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
        APP_ID,
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}

fn tracing_init() -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    let mut storage_dir =
        anyhow::Context::context(eframe::storage_dir(APP_ID), "Storage dir not found")?;
    storage_dir.push("log");

    let appender = tracing_appender::rolling::daily(storage_dir, "log");
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking_appender)
        .with_ansi(false)
        .init();
    tracing::info!("Tracing initialized and logging to file.");

    Ok(guard)
}
