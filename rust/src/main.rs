#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod catalog;
mod community;
mod custom;
mod diagnostics;
mod github_update;
mod inventory;
mod knowledge;
mod portable;
mod releases;
mod self_update;
mod service;
mod steam;
mod store;
mod ui;
mod winget;
use std::path::PathBuf;
fn main() {
    if let Err(e) = run() {
        #[cfg(windows)]
        if !std::env::args().any(|arg| {
            arg == "--scan-report"
                || arg == "--apply-self-update"
                || arg == "--self-update-report"
                || arg == "--inventory-report"
                || arg == "--store-report"
        }) {
            use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
            let text: Vec<u16> = e.encode_utf16().chain(Some(0)).collect();
            let title: Vec<u16> = "AppTide Beta".encode_utf16().chain(Some(0)).collect();
            unsafe {
                MessageBoxW(
                    std::ptr::null_mut(),
                    text.as_ptr(),
                    title.as_ptr(),
                    MB_OK | MB_ICONERROR,
                );
            }
        }
        eprintln!("{e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let value = |key: &str| {
        args.iter()
            .position(|a| a == key)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let demo = args.iter().any(|a| a == "--demo");
    if let Some(path) = value("--inventory-report") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&inventory::read()?).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Some(path) = value("--store-report") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&store::check_output(&store::run(false)?)?).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Some(path) = value("--apply-self-update") {
        return self_update::apply(std::path::Path::new(&path));
    }
    if let Some(path) = value("--self-update-report") {
        let report = serde_json::json!({"current":env!("CARGO_PKG_VERSION"),"available":self_update::check()?});
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap())
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let directory = value("--data-dir")
        .map(PathBuf::from)
        .unwrap_or_else(catalog::data_directory);
    let mut service = service::Service::open(directory, demo)?;
    if args.iter().any(|a| a == "--import-alpha") {
        service.import_alpha()?;
    }
    if let Some(path) = value("--scan-report") {
        let complete = service.scan()?;
        let report = serde_json::json!({"complete":complete,"records":service.catalog.apps.values().filter(|a|a.present).count(),"wingetRecords":service.catalog.apps.values().filter(|a|a.present&&a.winget_id.is_some()).count(),"eligibleUpdates":service.catalog.apps.values().filter(|a|catalog::eligible(a)).count(),"scans":service.catalog.scans,"warnings":service.warnings,"coverage":knowledge::coverage(&service.catalog)});
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap())
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let screenshot = value("--screenshot");
    if screenshot.is_some() && !demo {
        return Err("Screenshots require --demo to protect private inventory.".into());
    }
    let compact = args.iter().any(|a| a == "--compact");
    let scrolled = args.iter().any(|a| a == "--scrolled");
    let editor_preview = args.iter().any(|a| a == "--editor-preview");
    let catalogue_preview = args.iter().any(|a| a == "--catalogue-preview");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../../assets/apptide.png"))
                    .expect("Bundled AppTide icon"),
            )
            .with_inner_size(if compact {
                [1040.0, 700.0]
            } else {
                [1440.0, 900.0]
            })
            .with_min_inner_size([1040.0, 650.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "AppTide Beta",
        options,
        Box::new(move |cc| {
            let mut app = ui::Tide::new(service, &cc.egui_ctx);
            app.screenshot = screenshot;
            app.scrolled_demo = scrolled;
            if args.iter().any(|a| a == "--steam-preview") {
                app.steam_preview();
            }
            if args.iter().any(|a| a == "--directory-preview") {
                app.directory_preview();
            }
            if args.iter().any(|a| a == "--github-preview") {
                app.github_preview();
            }
            if catalogue_preview {
                app.catalogue_preview();
            }
            if editor_preview {
                app.editor_preview();
            }
            if demo && args.iter().any(|arg| arg == "--activity-preview") {
                app.activity_preview();
            }
            if demo && args.iter().any(|arg| arg == "--logs-preview") {
                app.logs_preview();
            }
            if demo && args.iter().any(|arg| arg == "--store-preview") {
                app.store_preview();
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string())
}
