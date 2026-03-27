#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(all(target_os = "windows", not(feature = "pcap")))]
use std::env;
mod app;
mod games;
mod themes;
mod ui;

const APP_ID: &str = "Stardb Exporter";

fn main() -> anyhow::Result<()> {
    let _guard = tracing_init()?;

    #[cfg(all(target_os = "windows", not(feature = "pcap")))]
    if !unsafe { windows::Win32::UI::Shell::IsUserAnAdmin().into() } {
        tracing::info!("Asking for admin permissions...");
        escalate_to_admin().expect("Error: failed to escalate privileges for pktmon version");
    }

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

#[cfg(all(target_os = "windows", not(feature = "pcap")))]
fn escalate_to_admin() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Console::GetConsoleWindow;
    use windows::Win32::UI::Shell::{
        SEE_MASK_NO_CONSOLE, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GW_OWNER, GetWindow, SW_SHOWNORMAL};
    use windows::core::PCWSTR;
    use windows::core::w;

    let args_str = env::args().skip(1).collect::<Vec<_>>().join(" ");

    let exe_path = env::current_exe()
        .expect("Failed to get current exe")
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let args = args_str.encode_utf16().chain(Some(0)).collect::<Vec<_>>();

    unsafe {
        let mut options = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NO_CONSOLE,
            hwnd: GetWindow(GetConsoleWindow(), GW_OWNER).unwrap_or(GetConsoleWindow()),
            lpVerb: w!("runas"),
            lpFile: PCWSTR(exe_path.as_ptr()),
            lpParameters: PCWSTR(args.as_ptr()),
            lpDirectory: PCWSTR::null(),
            nShow: SW_SHOWNORMAL.0,
            lpIDList: std::ptr::null_mut(),
            lpClass: PCWSTR::null(),
            dwHotKey: 0,
            ..Default::default()
        };

        ShellExecuteExW(&mut options)?;
    };

    // Exit the current process since we launched a new elevated one
    std::process::exit(0);
}
