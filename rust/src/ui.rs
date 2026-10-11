use crate::{
    catalog::{App, Catalog, actionable, eligible},
    service::{Plan, Service},
};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Stroke, Ui, Vec2};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
const BG: Color32 = Color32::from_rgb(247, 249, 251);
const NAV: Color32 = Color32::from_rgb(234, 240, 245);
const INK: Color32 = Color32::from_rgb(36, 53, 74);
const MUTED: Color32 = Color32::from_rgb(82, 102, 124);
const TEAL: Color32 = Color32::from_rgb(7, 109, 114);
const LINE: Color32 = Color32::from_rgb(220, 229, 237);
enum Command {
    Scan,
    CheckStore,
    ApplyStore(crate::store::Check),
    Execute(Plan),
    Pause(String, bool),
    Import,
    Watch(String, String),
    RefreshWatch(String),
    AutoPreference(Option<String>, bool),
    Automatic,
    Folder(String, bool),
    Custom(Option<String>, String, crate::custom::Source),
    StopTracking(String),
    CheckSelf,
    InstallSelf(crate::self_update::Release),
    StartupCheck(bool),
}
enum Event {
    State(Box<Catalog>, Vec<String>, String),
    Done(std::result::Result<(), String>),
    SelfChecked(std::result::Result<Option<crate::self_update::Release>, String>),
    RestartReady,
}
#[derive(Clone, Copy, PartialEq)]
enum Page {
    Updates,
    Store,
    Installed,
    Catalogue,
    Activity,
    Settings,
}
struct SourceEditor {
    id: Option<String>,
    manual: bool,
    name: String,
    source: crate::custom::Source,
    error: Option<String>,
}
#[derive(Default)]
struct LogViewer {
    files: Vec<String>,
    selected: usize,
    text: String,
}
pub struct Tide {
    catalog: Catalog,
    warnings: Vec<String>,
    page: Page,
    query: String,
    selected: Option<String>,
    show_system: bool,
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<Event>,
    cancel: Arc<AtomicBool>,
    busy: bool,
    phase: String,
    error: Option<String>,
    plan: Option<Plan>,
    store_plan: Option<crate::store::Check>,
    store_apply_rect: Option<Rect>,
    store_confirm_rect: Option<Rect>,
    demo: bool,
    directory: String,
    details_reset: bool,
    watch_input: String,
    watch_app: String,
    folder_input: String,
    coverage_filter: usize,
    source_editor: Option<SourceEditor>,
    own_release: Option<crate::self_update::Release>,
    own_status: String,
    restarting: bool,
    editor_save_rect: Option<Rect>,
    pub screenshot: Option<String>,
    frames: u32,
    screenshot_started: std::time::Instant,
    screenshot_requested: bool,
    pub scrolled_demo: bool,
    pub update_rect: Option<Rect>,
    update_all_rect: Option<Rect>,
    confirm_rect: Option<Rect>,
    logs_rect: Option<Rect>,
    log_viewer: Option<LogViewer>,
    pub uninstall_rect: Option<Rect>,
    pub list_rect: Option<Rect>,
}
pub fn style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(INK);
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.selection.bg_fill = Color32::from_rgb(212, 233, 233);
    visuals.selection.stroke = Stroke::new(1.0, TEAL);
    visuals.widgets.inactive.bg_fill = Color32::WHITE;
    visuals.widgets.inactive.weak_bg_fill = Color32::WHITE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(226, 240, 241);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(226, 240, 241);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, TEAL);
    visuals.widgets.active.bg_fill = Color32::from_rgb(212, 233, 233);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(212, 233, 233);
    ctx.set_visuals(visuals);
    ctx.style_mut_of(egui::Theme::Light, |s| {
        s.spacing.item_spacing = Vec2::new(10.0, 10.0);
        s.spacing.button_padding = Vec2::new(14.0, 9.0);
        s.spacing.interact_size = Vec2::new(40.0, 36.0);
        s.text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
        s.text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
        s.text_styles
            .insert(egui::TextStyle::Heading, FontId::proportional(24.0));
    });
    // Use the user's installed Windows typeface; do not redistribute Microsoft fonts.
    if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/segoeui.ttf") {
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("Segoe UI".into(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "Segoe UI".into());
        ctx.set_fonts(fonts);
    }
}
impl Tide {
    pub fn store_preview(&mut self) {
        if self.demo {
            self.page = Page::Store;
            self.catalog.extra.insert(
                "storeCheck".into(),
                serde_json::to_value(crate::store::demo_check()).unwrap(),
            );
        }
    }
    fn store_page(&mut self, ui: &mut Ui) {
        ui.label("Check and update apps through Microsoft's installed Store client.");
        ui.label("Store updates run separately from the WinGet queue. Microsoft controls the batch and its dependencies.");
        ui.add_space(10.0);
        let check = self
            .catalog
            .extra
            .get("storeCheck")
            .and_then(|v| serde_json::from_value::<crate::store::Check>(v.clone()).ok());
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.busy, egui::Button::new("Check Store updates"))
                .clicked()
            {
                self.send(Command::CheckStore, "Checking Microsoft Store updates…");
            }
            let button = ui.add_enabled(
                !self.busy
                    && check
                        .as_ref()
                        .is_some_and(|c| c.error.is_none() && !c.updates.is_empty()),
                egui::Button::new(RichText::new("Update Store apps…").color(Color32::WHITE))
                    .fill(TEAL),
            );
            self.store_apply_rect = Some(button.rect);
            if button.clicked() {
                self.store_plan = check.clone();
            }
            if ui.button("Open Store Downloads").clicked() {
                open_store();
            }
        });
        ui.add_space(12.0);
        let Some(check) = check else {
            ui.label("Check for available Store updates to see the list here.");
            return;
        };
        if let Some(error) = &check.error {
            ui.colored_label(Color32::from_rgb(150, 80, 10), error);
            return;
        }
        ui.label(
            RichText::new(format!(
                "{} updates reported · Checked {}",
                check.updates.len(),
                check.checked_at
            ))
            .color(MUTED),
        );
        if check.updates.is_empty() {
            ui.heading("No Store updates reported");
            ui.label("Availability can change. You can also check in Store Downloads.");
            return;
        }
        ui.label("Versions below are installed versions reported by Store. Target versions are chosen by Microsoft at update time.");
        ui.add_space(10.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for app in &check.updates {
                ui.label(RichText::new(&app.name).strong());
                ui.label(format!("{} · Installed {}", app.publisher, app.version));
                ui.separator();
            }
        });
    }
    fn show_logs(&mut self) {
        if self.demo {
            self.log_viewer=Some(LogViewer { files:vec!["receipts/synthetic-example.log".into()],selected:0,text:"Synthetic download result\nHTTP 404: installer file not found\nThis app failed. Remaining independent updates continued.\nNo private file was read.".into() });
            return;
        }
        self.log_viewer = Some(
            match crate::diagnostics::log_files(std::path::Path::new(&self.directory)) {
                Ok(files) => {
                    let text = files
                        .first()
                        .map(|file| {
                            crate::diagnostics::read_log(
                                std::path::Path::new(&self.directory),
                                file,
                            )
                            .unwrap_or_else(|e| format!("Cannot read log: {e}"))
                        })
                        .unwrap_or_else(|| "No diagnostic files have been recorded yet.".into());
                    LogViewer {
                        files,
                        selected: 0,
                        text,
                    }
                }
                Err(e) => LogViewer {
                    text: format!("Cannot list logs: {e}"),
                    ..Default::default()
                },
            },
        );
    }
    pub fn logs_preview(&mut self) {
        if self.demo {
            self.activity_preview();
            self.show_logs();
        }
    }
    pub fn activity_preview(&mut self) {
        if self.demo {
            self.page = Page::Activity;
            self.catalog.extra.insert("lastBatch".into(), serde_json::json!({"status":"attention", "summary":"Queue finished with issues: 4 verified, 1 skipped, 1 need review, 0 not run.", "detail":"Example helper needs review. The remaining independent updates were attempted. Open its local log for details."}));
            self.catalog.receipts = vec![
                serde_json::json!({"name":"Aster Notes","kind":"update","status":"verified","startedAt":"Synthetic demonstration","detail":"Confirmed by a fresh inventory.","exitCode":0}),
                serde_json::json!({"name":"Example helper","kind":"update","status":"unverified","startedAt":"Synthetic demonstration","detail":"Multiple installed registrations match this package (1.0, 2.0). Review these installations before retrying; no automatic retry will run.","exitCode":0,"logFile":"synthetic-example.log"}),
            ];
        }
    }
    pub fn editor_preview(&mut self) {
        if self.demo {
            self.page = Page::Catalogue;
            self.source_editor=Some(SourceEditor{id:None,manual:true,name:"Orchid utility".into(),source:crate::custom::Source{url:"https://example.org/downloads".into(),instructions:"Open Help > Check for updates. Keep the same portable folder when upgrading.".into(),manual_version:"1.2.0".into(),..Default::default()},error:None});
        }
    }
    pub fn catalogue_preview(&mut self) {
        if self.demo {
            self.page = Page::Catalogue;
            self.selected = Some("recipe-demo".into());
            self.coverage_filter = 1;
        }
    }
    pub fn new(service: Service, ctx: &egui::Context) -> Self {
        style(ctx);
        let catalog = service.catalog.clone();
        let demo = service.demo;
        let directory = service.directory.display().to_string();
        let restart_error = if demo {
            None
        } else {
            std::fs::read(service.directory.join("self-update-result.json"))
                .ok()
                .filter(|b| b.len() < 65536)
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .filter(|v| v["status"] == "failed")
                .and_then(|v| {
                    v["detail"]
                        .as_str()
                        .map(|s| format!("Previous AppTide update: {s}"))
                })
        };
        let cancel = service.cancel.clone();
        let (tx, commands) = mpsc::channel();
        let (events, rx) = mpsc::channel();
        let context = ctx.clone();
        if !demo && catalog.settings["checkAppTideAtStartup"] != false {
            let events = events.clone();
            let context = context.clone();
            std::thread::spawn(move || {
                let result = crate::self_update::check();
                let _ = events.send(Event::SelfChecked(result));
                context.request_repaint();
            });
        }
        std::thread::spawn(move || {
            let mut service = service;
            service.diagnostic("startup", "Application worker ready");
            while let Ok(command) = commands.recv() {
                let operation = match &command {
                    Command::Scan => "scan",
                    Command::CheckStore => "Store check",
                    Command::ApplyStore(_) => "Store updates",
                    Command::Execute(_) => "manual queue",
                    Command::Automatic => "automatic queue",
                    Command::CheckSelf => "self-update check",
                    Command::InstallSelf(_) => "self-update installation",
                    _ => "catalogue or settings action",
                };
                service.diagnostic("started", operation);
                let result = match command {
                    Command::Scan => {
                        service.cancel.store(false, Ordering::Relaxed);
                        service.scan().map(|_| {
                            if let Err(error) = service.check_store() {
                                service.warnings.push(error);
                            }
                        })
                    }
                    Command::CheckStore => service.check_store().map(|_| ()),
                    Command::ApplyStore(preview) => service.execute_store(preview),
                    Command::Execute(plan) => service.execute(plan, |phase, cat| {
                        let _ =
                            events.send(Event::State(Box::new(cat.clone()), vec![], phase.into()));
                        context.request_repaint();
                    }),
                    Command::Pause(id, value) => {
                        if let Some(a) = service.catalog.apps.get_mut(&id) {
                            a.excluded = value;
                        }
                        service.save()
                    }
                    Command::Import => service.import_alpha(),
                    Command::Watch(id, repo) => service.watch(&id, &repo),
                    Command::RefreshWatch(id) => {
                        service.cancel.store(false, Ordering::Relaxed);
                        service.refresh_releases(Some(&id));
                        service.save()
                    }
                    Command::AutoPreference(id, enabled) => {
                        service.automatic_preference(id.as_deref(), enabled)
                    }
                    Command::Folder(folder, remove) => service.add_folder(&folder, remove),
                    Command::Custom(id, name, source) => {
                        service.custom_source(id.as_deref(), &name, source)
                    }
                    Command::StopTracking(id) => {
                        if let Some(a) = service.catalog.apps.get_mut(&id)
                            && a.kind == "manual"
                        {
                            a.present = false;
                        }
                        service.save()
                    }
                    Command::CheckSelf => {
                        let result = crate::self_update::check();
                        let _ = events.send(Event::SelfChecked(result));
                        Ok(())
                    }
                    Command::StartupCheck(enabled) => {
                        service.catalog.settings["checkAppTideAtStartup"] = enabled.into();
                        service.save()
                    }
                    Command::InstallSelf(release) => {
                        if service.demo {
                            Err("Self-updates are disabled in demo mode".into())
                        } else {
                            crate::self_update::stage(&release, &service.directory).and_then(
                                |plan| {
                                    crate::self_update::launch_helper(&plan)?;
                                    let _ = events.send(Event::RestartReady);
                                    Ok(())
                                },
                            )
                        }
                    }
                    Command::Automatic => service.automatic(|phase, cat| {
                        let _ =
                            events.send(Event::State(Box::new(cat.clone()), vec![], phase.into()));
                        context.request_repaint();
                    }),
                };
                service.diagnostic(
                    if result.is_ok() { "finished" } else { "error" },
                    &format!(
                        "{operation}: {}",
                        result
                            .as_ref()
                            .err()
                            .map(String::as_str)
                            .unwrap_or("completed")
                    ),
                );
                let _ = events.send(Event::State(
                    Box::new(service.catalog.clone()),
                    service.warnings.clone(),
                    String::new(),
                ));
                let _ = events.send(Event::Done(result));
                context.request_repaint();
            }
        });
        let selected = catalog
            .apps
            .values()
            .filter(|a| eligible(a))
            .min_by_key(|a| a.name.clone())
            .map(|a| a.uid.clone());
        let own_status = if demo {
            "Demo: self-update checks disabled".into()
        } else if catalog.settings["checkAppTideAtStartup"] == false {
            "Startup checks disabled".into()
        } else {
            "Checking GitHub for AppTide updates...".into()
        };
        Self {
            catalog,
            warnings: vec![],
            page: Page::Updates,
            query: String::new(),
            selected,
            show_system: false,
            tx,
            rx,
            cancel,
            busy: false,
            phase: String::new(),
            error: restart_error,
            plan: None,
            store_plan: None,
            store_apply_rect: None,
            store_confirm_rect: None,
            demo,
            directory,
            details_reset: false,
            watch_input: String::new(),
            watch_app: String::new(),
            folder_input: String::new(),
            coverage_filter: 0,
            source_editor: None,
            own_release: None,
            own_status,
            restarting: false,
            editor_save_rect: None,
            screenshot: None,
            frames: 0,
            screenshot_started: std::time::Instant::now(),
            screenshot_requested: false,
            scrolled_demo: false,
            update_rect: None,
            update_all_rect: None,
            confirm_rect: None,
            logs_rect: None,
            log_viewer: None,
            uninstall_rect: None,
            list_rect: None,
        }
    }
    fn send(&mut self, command: Command, phase: &str) {
        if self.busy || self.restarting {
            return;
        }
        self.busy = true;
        self.phase = phase.into();
        self.error = None;
        if self.tx.send(command).is_err() {
            self.error = Some("Worker stopped. Restart AppTide Beta.".into());
            self.busy = false;
        }
    }
    fn prepare(&mut self, kind: &str, ids: Vec<String>) {
        match crate::service::prepare(&self.catalog, kind, &ids) {
            Ok(plan) => self.plan = Some(plan),
            Err(e) => self.error = Some(e),
        }
    }
    fn nav(&mut self, ui: &mut Ui) {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::hover());
            let p = ui.painter();
            let points = vec![
                r.left_bottom() + Vec2::new(2.0, -5.0),
                r.center_top() + Vec2::new(-4.0, 3.0),
                r.center_top() + Vec2::new(4.0, 3.0),
                r.right_bottom() + Vec2::new(-2.0, -5.0),
            ];
            p.add(egui::Shape::line(points, Stroke::new(2.0, TEAL)));
            p.line_segment(
                [
                    r.left_bottom() + Vec2::new(9.0, -6.0),
                    r.right_bottom() + Vec2::new(-8.0, -4.0),
                ],
                Stroke::new(2.0, TEAL),
            );
            ui.label(RichText::new("AppTide").size(23.0).strong());
        });
        ui.add_space(8.0);
        ui.colored_label(TEAL, concat!("Rust ", env!("CARGO_PKG_VERSION")));
        ui.add_space(30.0);
        let count = self.catalog.apps.values().filter(|a| eligible(a)).count();
        for (page, label) in [
            (Page::Updates, format!("Updates                 {count}")),
            (Page::Store, "Microsoft Store".into()),
            (Page::Installed, "Installed apps".into()),
            (Page::Catalogue, "My catalogue".into()),
            (Page::Activity, "Activity".into()),
            (Page::Settings, "Settings".into()),
        ] {
            let selected = self.page == page;
            let b =
                egui::Button::new(RichText::new(label).color(if selected { TEAL } else { INK }))
                    .fill(if selected {
                        Color32::from_rgb(211, 230, 232)
                    } else {
                        NAV
                    })
                    .stroke(Stroke::NONE);
            if ui.add_sized([170.0, 43.0], b).clicked() {
                self.page = page;
                self.query.clear();
            }
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.label(
                RichText::new("Free. Open source. Yours.")
                    .size(12.0)
                    .color(MUTED),
            );
            ui.label(
                RichText::new("Your catalogue stays here.")
                    .size(12.0)
                    .color(MUTED),
            );
        });
    }
    fn detail(&mut self, ui: &mut Ui) {
        let Some(a) = self
            .selected
            .as_ref()
            .and_then(|id| self.catalog.apps.get(id))
            .cloned()
        else {
            ui.add_space(30.0);
            ui.heading("Select an app");
            ui.label("Its version, source and available actions will appear here.");
            return;
        };
        ui.add(egui::Label::new(RichText::new(&a.name).size(21.0).strong()).truncate())
            .on_hover_text(&a.name);
        ui.add(
            egui::Label::new(
                RichText::new(if a.publisher.trim().is_empty() {
                    "Vendor not reported"
                } else {
                    &a.publisher
                })
                .color(MUTED),
            )
            .truncate(),
        )
        .on_hover_text(&a.publisher);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Installed").color(MUTED));
            ui.label(
                RichText::new(if a.version.trim().is_empty() {
                    "Version not reported"
                } else {
                    &a.version
                })
                .strong(),
            );
        });
        if !a.available.is_empty() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Available").color(MUTED));
                ui.colored_label(TEAL, &a.available);
            });
        } else if let Some(tag) = a.extra.get("watch").and_then(|w| w["latest"].as_str()) {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Release tag").color(MUTED));
                ui.colored_label(TEAL, tag);
            });
        }
        ui.add_space(10.0);
        let external = !a.protected
            && !a.ambiguous
            && (crate::releases::repository(&a).is_some()
                || a.knowledge.is_some()
                || crate::custom::get(&a).is_some_and(|s| !s.url.is_empty()));
        let update = ui.add_enabled(
            !self.busy && (eligible(&a) || external),
            egui::Button::new(
                RichText::new(if eligible(&a) {
                    "Update this app"
                } else if crate::releases::repository(&a).is_some() {
                    "Open release page"
                } else if external {
                    "Open update instructions"
                } else {
                    "Update this app"
                })
                .color(Color32::WHITE),
            )
            .fill(TEAL)
            .min_size(Vec2::new(ui.available_width(), 40.0)),
        );
        self.update_rect = Some(update.rect);
        if update.clicked() {
            if eligible(&a) {
                self.prepare("update", vec![a.uid.clone()]);
            } else if let Some(repo) = crate::releases::repository(&a) {
                open_url(&format!("https://github.com/{repo}/releases"));
            } else if let Some(source) = crate::custom::get(&a).filter(|s| !s.url.is_empty()) {
                open_url(&source.url);
            } else if let Some(k) = &a.knowledge {
                open_url(&k.source);
            }
        }
        let remove = ui.add_enabled(
            !self.busy && actionable(&a),
            egui::Button::new(RichText::new("Uninstall app").color(Color32::from_rgb(163, 42, 52)))
                .min_size(Vec2::new(ui.available_width(), 38.0)),
        );
        self.uninstall_rect = Some(remove.rect);
        if remove.clicked() {
            self.prepare("uninstall", vec![a.uid.clone()]);
        }
        if !actionable(&a) {
            ui.label(
                RichText::new(if a.protected {
                    "Windows component protected from changes."
                } else if a.ambiguous {
                    "Multiple installed versions. Resolve in Windows Settings."
                } else {
                    "Use the update route below. Removal is available in Windows Settings."
                })
                .size(12.0)
                .color(MUTED),
            );
            if ui.button("Open Windows installed apps").clicked() {
                open_settings();
            }
        }
        ui.add_space(8.0);
        ui.separator();
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt(("details", &a.uid))
            .auto_shrink([false, false]);
        if self.details_reset {
            scroll = scroll.vertical_scroll_offset(0.0);
            self.details_reset = false;
        }
        scroll.show(ui,|ui|{
            if a.kind=="appx" && a.extra.get("signatureKind").and_then(|v|v.as_str())==Some("Store") && ui.button("Manage Microsoft Store updates").clicked(){self.page=Page::Store;}
            if let Some(sources)=a.extra.get("metadataSources") {
                field(ui,"Vendor source",sources["publisher"].as_str().unwrap_or("Not reported"));
                field(ui,"Version source",sources["version"].as_str().unwrap_or("Not reported"));
            }
            if let Some(arch)=a.extra.get("architecture").and_then(|v|v.as_str()){field(ui,"Architecture",arch);}
            if ui.add_enabled(!self.busy,egui::Button::new("Edit my catalogue entry")).clicked(){
                self.source_editor=Some(SourceEditor{id:Some(a.uid.clone()),manual:a.kind=="manual",name:a.name.clone(),source:crate::custom::get(&a).unwrap_or_else(||crate::custom::Source{manual_version:a.version.clone(),url:a.knowledge.as_ref().map(|k|k.source.clone()).unwrap_or_default(),repository:crate::releases::repository(&a).unwrap_or("").into(),..Default::default()}),error:None});
            }
            if let Some(source)=crate::custom::get(&a){field(ui,"Your update notes",&source.instructions);if !source.url.is_empty() && ui.link("Open your saved source").clicked(){open_url(&source.url);}}
            if a.kind=="manual" {ui.label("Manually tracked; installation is not detected.");if ui.add_enabled(!self.busy && a.present,egui::Button::new("Stop tracking this entry")).clicked(){self.send(Command::StopTracking(a.uid.clone()),"Keeping entry as history…");}}
            ui.add_space(12.0);field(ui,"Update route",crate::knowledge::route(&a));
            if let Some(k)=&a.knowledge {
                ui.label(&k.instructions);field(ui,"Detection evidence",&k.evidence);
                if ui.link("Official update instructions").clicked(){open_url(&k.source);}
            } else if a.winget_id.is_none() {ui.label("No reviewed recipe yet. Version sightings remain private and never become installer commands.");}

            let mut excluded=a.excluded;if ui.add_enabled(!self.busy,egui::Checkbox::new(&mut excluded,"Pause updates for this app")).changed(){self.send(Command::Pause(a.uid.clone(),excluded),"Saving preference…");}
            ui.add_space(12.0);field(ui,"Update source",a.winget_id.as_deref().unwrap_or("Not matched"));field(ui,"Evidence",&a.evidence);
            field(ui,"Local knowledge",&format!("{} scans · {} log sightings",a.observations,a.log_mentions));
            if let Some(l)=a.extra.get("learning"){ui.label(RichText::new(format!("{} versions / {} verified operations",l["versionSightings"],l["verifiedOperations"])).small().color(MUTED));}
            if let Some(notice)=a.extra.get("identityNotice").and_then(|v|v.as_str()){ui.label(notice);}

            let mut automatic=a.auto_update && a.extra.get("automaticConsent")==Some(&serde_json::json!(2));
            if ui.add_enabled(!self.busy && actionable(&a),egui::Checkbox::new(&mut automatic,"Allow automatic WinGet updates")).changed(){self.send(Command::AutoPreference(Some(a.uid.clone()),automatic),"Saving automatic update choice…");}
            ui.label(RichText::new("Also requires the global switch in Settings.").small().color(MUTED));
            ui.add_space(12.0);ui.label(RichText::new("GitHub release watch").strong());
            if self.watch_app!=a.uid {self.watch_app=a.uid.clone();self.watch_input=crate::releases::repository(&a).unwrap_or("").into();}
            ui.add_sized([ui.available_width(),32.0],egui::TextEdit::singleline(&mut self.watch_input).hint_text("owner/repository"));
            ui.horizontal(|ui|{
                if ui.add_enabled(!self.busy,egui::Button::new("Save watch")).clicked(){self.send(Command::Watch(a.uid.clone(),self.watch_input.clone()),"Saving and checking release watch…");}
                if ui.add_enabled(!self.busy && crate::releases::repository(&a).is_some(),egui::Button::new("Refresh")).clicked(){self.send(Command::RefreshWatch(a.uid.clone()),"Checking stable release…");}
            });
            ui.label(RichText::new("Tracks releases only. Review the edition and installer yourself.").small().color(MUTED));
            if let Some(w)=a.extra.get("watch") {field(ui,"Release comparison",w["status"].as_str().unwrap_or("Not checked"));if let Some(v)=w["latest"].as_str(){field(ui,"Latest stable tag",v);}
                if let Some(t)=w["checkedAt"].as_str(){field(ui,"Last successful check",t);}}
            if let Some(repo)=crate::releases::repository(&a) && ui.link("Open project releases").clicked(){open_url(&format!("https://github.com/{repo}/releases"));}
            ui.add_space(8.0);ui.label(RichText::new("Observed versions").strong());for v in a.versions.iter().rev(){ui.horizontal(|ui|{ui.label(&v.version);ui.label(RichText::new(v.seen_at.get(..10).unwrap_or(&v.seen_at)).size(12.0).color(MUTED));});}
            if !a.location.is_empty(){ui.add_space(8.0);field(ui,"Location",&a.location);}
        });
    }
    fn apps(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_sized(
                [ui.available_width() - 165.0, 36.0],
                egui::TextEdit::singleline(&mut self.query).hint_text("Find an app"),
            );
            ui.checkbox(&mut self.show_system, "System components");
        });
        ui.add_space(8.0);
        let query = self.query.to_lowercase();
        if self.page != Page::Updates {
            egui::ComboBox::from_id_salt("coverage-filter")
                .selected_text(
                    [
                        "All update routes",
                        "Beyond WinGet",
                        "Needs research",
                        "Release watches",
                    ][self.coverage_filter],
                )
                .show_ui(ui, |ui| {
                    for (i, label) in [
                        "All update routes",
                        "Beyond WinGet",
                        "Needs research",
                        "Release watches",
                    ]
                    .iter()
                    .enumerate()
                    {
                        ui.selectable_value(&mut self.coverage_filter, i, *label);
                    }
                });
        }
        let mut apps: Vec<_> = self
            .catalog
            .apps
            .values()
            .filter(|a| {
                (self.page == Page::Catalogue || a.present)
                    && (self.page != Page::Installed || a.kind != "manual")
                    && (self.show_system || !a.protected)
                    && (self.page != Page::Updates
                        || !a.available.is_empty()
                        || crate::releases::newer(a))
                    && (self.page == Page::Updates
                        || match self.coverage_filter {
                            1 => {
                                a.winget_id.is_none()
                                    && (a.knowledge.is_some() || crate::custom::get(a).is_some())
                            }
                            2 => crate::knowledge::route(a) == "Needs research",
                            3 => crate::releases::repository(a).is_some(),
                            _ => true,
                        })
                    && (query.is_empty()
                        || a.name.to_lowercase().contains(&query)
                        || a.publisher.to_lowercase().contains(&query)
                        || a.winget_id
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&query)
                        || crate::knowledge::route(a).to_lowercase().contains(&query))
            })
            .cloned()
            .collect();
        apps.sort_by_key(|a| a.name.to_lowercase());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} app records", apps.len()))
                    .size(12.0)
                    .color(MUTED),
            );
        });
        ui.separator();
        if apps.is_empty() {
            ui.add_space(30.0);
            ui.heading(if self.catalog.scans == 0 {
                "Let's see what's installed."
            } else if !self.query.is_empty() {
                "No matching apps"
            } else {
                "No updates in this view"
            });
            ui.label(if self.catalog.scans==0{"Check for updates to build your private catalogue, or import your alpha catalogue in Settings."}else{"Only apps with a known update source can report updates. Check Installed apps for coverage."});
            return;
        }
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt(("apps", self.page as u8))
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .auto_shrink([false, false]);
        if self.scrolled_demo {
            scroll = scroll.vertical_scroll_offset(5000.0);
        }
        let output = scroll.show_rows(ui, 66.0, apps.len(), |ui, range| {
            for index in range {
                let a = &apps[index];
                let selected = self.selected.as_ref() == Some(&a.uid);
                let response = app_row(ui, a, selected);
                if response.clicked() {
                    self.selected = Some(a.uid.clone());
                    self.details_reset = true;
                }
            }
        });
        self.list_rect = Some(output.inner_rect);
    }
    fn activity(&mut self, ui: &mut Ui) {
        let button = ui.button("View local logs");
        self.logs_rect = Some(button.rect);
        if button.clicked() {
            self.show_logs();
        }
        ui.label("Logs contain app events; receipts contain WinGet output; batches contain queue plans and stop reasons. Keep these files private.");
        if self.catalog.receipts.is_empty() {
            ui.add_space(30.0);
            ui.heading("A history you can check.");
            ui.label("Updates and removals will appear here with their verification result.");
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for r in self.catalog.receipts.iter().rev().take(100) {
                ui.label(RichText::new(r["name"].as_str().unwrap_or("App")).strong());
                ui.horizontal(|ui| {
                    ui.label(r["kind"].as_str().unwrap_or(""));
                    ui.colored_label(
                        if r["status"] == "verified" {
                            TEAL
                        } else {
                            Color32::from_rgb(150, 80, 10)
                        },
                        r["status"].as_str().unwrap_or(""),
                    );
                    ui.label(r["startedAt"].as_str().unwrap_or(""));
                });
                ui.label(r["detail"].as_str().unwrap_or(""));
                if let Some(code) = r["exitCode"].as_i64() {
                    ui.label(format!(
                        "{} exit code: {code} (0x{:08X})",
                        r["provider"].as_str().unwrap_or("WinGet"),
                        code as u32
                    ));
                }
                if let Some(file) = r["logFile"].as_str() {
                    ui.label(format!("Output: receipts/{file}"));
                }
                ui.separator();
            }
        });
    }
    fn settings(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui,|ui|{
        ui.heading("AppTide updates");ui.label(&self.own_status);
        let mut startup=self.catalog.settings["checkAppTideAtStartup"]!=false;
        if ui.add_enabled(!self.busy,egui::Checkbox::new(&mut startup,"Check GitHub when AppTide opens")).changed(){self.send(Command::StartupCheck(startup),"Saving startup check preference...");}
        if ui.add_enabled(!self.busy && !self.demo,egui::Button::new("Check AppTide version")).clicked(){self.send(Command::CheckSelf,"Checking AppTide releases...");}
        ui.label("Beta channel. Downloads are verified against GitHub's SHA-256 digest. Updating restarts AppTide, keeps a backup of the old package files, and leaves your catalogue in place.");ui.add_space(18.0);
        ui.heading("Your private catalogue");ui.label("The Rust beta uses a separate catalogue. Import copies your alpha history and preferences; the original stays in place.");
        if ui.add_enabled(!self.busy&&self.catalog.apps.is_empty()&&!self.demo,egui::Button::new("Import alpha catalogue")).clicked(){self.send(Command::Import,"Importing catalogue…");}
        ui.add_space(12.0);field(ui,"Data folder",&self.directory);ui.add_space(20.0);ui.heading("This beta's scope");
        ui.label("Reviewed public recipes identify update owners and release sources. Your installed inventory, histories and receipts stay on this PC. Unknown apps remain research candidates; observations never authorize arbitrary installers.");
        ui.add_space(16.0);ui.heading("Automatic updates while open");
        let mut automatic=self.catalog.settings["automatic"]==true && self.catalog.settings["automaticConsent"]==2;
        if ui.add_enabled(!self.busy,egui::Checkbox::new(&mut automatic,"Check hourly and update opted-in WinGet apps")).changed(){self.send(Command::AutoPreference(None,automatic),"Saving automatic update choice…");}
        ui.label("Off by default. Enable individual apps in their details. The first check is one hour after enabling. Failed or unverified operations block automatic retries until a manual operation verifies successfully. Closing AppTide stops scheduling; no service or startup task is installed.");
        ui.add_space(16.0);ui.heading("Portable app folders");
        ui.label("Read executable version metadata in folders you choose, up to two subfolder levels. Nothing is launched. Portable discoveries do not grant install or removal permission.");
        ui.add_sized([ui.available_width().min(600.0),36.0],egui::TextEdit::singleline(&mut self.folder_input).hint_text(r"C:\Tools"));
        if ui.add_enabled(!self.busy && !self.folder_input.is_empty(),egui::Button::new("Add folder")).clicked(){self.send(Command::Folder(self.folder_input.clone(),false),"Saving portable folder…");}
        let folders:Vec<String>=serde_json::from_value(self.catalog.settings["portableFolders"].clone()).unwrap_or_default();
        for folder in folders {ui.horizontal(|ui|{ui.label(&folder);if ui.add_enabled(!self.busy,egui::Button::new("Remove folder")).clicked(){self.send(Command::Folder(folder.clone(),true),"Saving portable folders…");}});}
        ui.label("Use Check for updates after changing folders. Scan limits: 10 roots, 1,000 folders, 10,000 files and 500 executables. Links and junctions are skipped.");
        ui.add_space(20.0);ui.heading("Native by design");ui.label("A Rust core and a custom egui immediate-mode interface. Lists render only visible rows. Scans and installers run on a worker thread. The UI sleeps when idle.");if ui.link("AppTide source and issues").clicked() { open_project(); }
    });
    }
    pub fn draw(&mut self, ui: &mut Ui) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::State(cat, warnings, phase) => {
                    self.catalog = *cat;
                    self.warnings = warnings;
                    self.phase = phase;
                }
                Event::Done(result) => {
                    self.busy = false;
                    if let Err(e) = result {
                        // Queue summaries already persist above the list. Avoid repeating a long
                        // error banner that pushes the useful content below the viewport.
                        let summarized = self
                            .catalog
                            .extra
                            .get("lastBatch")
                            .and_then(|b| b["summary"].as_str())
                            .is_some_and(|summary| e.starts_with(summary));
                        self.error = if summarized { None } else { Some(e) };
                    }
                    if self.selected.is_none() {
                        self.selected = self
                            .catalog
                            .apps
                            .values()
                            .filter(|a| eligible(a))
                            .min_by_key(|a| a.name.clone())
                            .map(|a| a.uid.clone());
                    }
                }
                Event::SelfChecked(result) => match result {
                    Ok(Some(release)) => {
                        self.own_status = format!("AppTide {} is available", release.version);
                        self.own_release = Some(release);
                    }
                    Ok(None) => {
                        self.own_status = "No newer compatible AppTide release found".into();
                        self.own_release = None;
                    }
                    Err(e) => self.own_status = format!("AppTide update check unavailable: {e}"),
                },
                Event::RestartReady => self.restarting = true,
            }
        }
        let ctx = ui.ctx().clone();
        if !self.demo && !self.restarting && self.catalog.settings["automatic"] == true {
            ctx.request_repaint_after(Duration::from_secs(30));
            if !self.busy
                && self.plan.is_none()
                && self.store_plan.is_none()
                && self.source_editor.is_none()
                && crate::service::automatic_due(&self.catalog)
            {
                self.send(Command::Automatic, "Checking opted-in automatic updates…");
            }
        }
        if self.busy && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.error=Some("Wait for the current operation to finish before closing. Stop queue skips remaining apps.".into());
        }
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0, BG);
        let nav = Rect::from_min_max(rect.min, Pos2::new(rect.min.x + 205.0, rect.max.y));
        ui.painter().rect_filled(nav, 0, NAV);
        ui.painter().line_segment(
            [nav.right_top(), nav.right_bottom()],
            Stroke::new(1.0, LINE),
        );
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(nav.shrink2(Vec2::new(17.0, 22.0))),
            |ui| self.nav(ui),
        );
        let content = Rect::from_min_max(
            Pos2::new(nav.right() + 28.0, rect.top() + 28.0),
            rect.max - Vec2::new(28.0, 18.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(content),|ui|{
            ui.horizontal(|ui|{ui.vertical(|ui|{ui.heading(RichText::new(match self.page{Page::Updates=>"Updates",Page::Store=>"Microsoft Store",Page::Installed=>"Installed apps",Page::Catalogue=>"My catalogue",Page::Activity=>"Activity",Page::Settings=>"Settings"}).size(30.0));ui.label(RichText::new("A little maintenance. Everything in one place.").color(MUTED));});
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{if self.page==Page::Catalogue && ui.add_enabled(!self.busy,egui::Button::new("Add custom app")).clicked(){self.source_editor=Some(SourceEditor{id:None,manual:true,name:String::new(),source:Default::default(),error:None});}
                    if self.page==Page::Updates{let ids:Vec<_>=self.catalog.apps.values().filter(|a|eligible(a)).map(|a|a.uid.clone()).collect();let button=ui.add_enabled(!self.busy&&!ids.is_empty(),egui::Button::new(RichText::new(format!("Update all ({})",ids.len())).color(Color32::WHITE)).fill(TEAL));self.update_all_rect=Some(button.rect);if button.clicked(){self.prepare("update",ids);}}
                    if self.page!=Page::Store && ui.add_enabled(!self.busy,egui::Button::new("Check for updates")).clicked(){self.send(Command::Scan,"Reading installed software and WinGet sources…");}
                });
            });ui.add_space(18.0);
            if let Some(release)=self.own_release.clone(){ui.horizontal(|ui|{ui.colored_label(TEAL,format!("AppTide {} is ready",release.version));if ui.add_enabled(!self.busy,egui::Button::new("Update AppTide and restart")).clicked(){self.send(Command::InstallSelf(release),"Downloading and verifying AppTide...");}});}
            if self.demo{ui.colored_label(TEAL,"DEMO  ·  Synthetic apps. Update and uninstall actions are simulated.");}
            if let Some(error)=&self.error{ui.colored_label(Color32::from_rgb(161,43,42),error);}
            if !self.busy && let Some(batch)=self.catalog.extra.get("lastBatch") {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(if batch["status"]=="complete" {TEAL} else {Color32::from_rgb(137,80,14)}, batch["summary"].as_str().unwrap_or("Previous queue needs review."));
                    if ui.link("View Activity").clicked() { self.page=Page::Activity; }
                });
                if self.page==Page::Activity && let Some(detail)=batch["detail"].as_str() && !detail.is_empty() {ui.label(detail);}
            }
            for warning in &self.warnings{ui.colored_label(Color32::from_rgb(137,80,14),RichText::new(warning).size(12.0));}
            if self.busy {ui.horizontal(|ui|{ui.spinner();ui.label(&self.phase);if !self.phase.starts_with("Microsoft Store is updating") && ui.button("Stop queue").clicked(){self.cancel.store(true,Ordering::Relaxed);}});}
            let learned=self.catalog.apps.values().filter(|a|a.present && a.winget_id.is_none() && a.knowledge.is_some()).count();let present=self.catalog.apps.values().filter(|a|a.present).count();let matched=self.catalog.apps.values().filter(|a|a.present&&a.winget_id.is_some()).count();
            ui.horizontal(|ui|{ui.label(RichText::new(format!("{present} app records  ·  {matched} WinGet / {learned} other known routes  ·  {} catalogue scans",self.catalog.scans)).size(12.0).color(MUTED));});ui.separator();ui.add_space(10.0);
            if self.page==Page::Updates {
                ui.horizontal_wrapped(|ui| {
                    let count=self.catalog.extra.get("storeCheck").and_then(|v|v["updates"].as_array()).map(Vec::len);
                    let failed=self.catalog.extra.get("storeCheck").is_some_and(|v|v["error"].is_string());
                    ui.label(if failed {"Microsoft Store: check unavailable".into()}else{match count {Some(n)=>format!("Microsoft Store: {n} updates reported"),None=>"Microsoft Store: not checked".into()}});
                    if ui.link("Manage Store updates").clicked(){self.page=Page::Store;}
                }); ui.add_space(8.0);
            }
            if self.page==Page::Store {self.store_page(ui);}else if matches!(self.page,Page::Activity){self.activity(ui);}else if self.page==Page::Settings{self.settings(ui);}else{
                let body=ui.available_rect_before_wrap();let details_width=290.0;let split=body.right()-details_width-20.0;
                let list=Rect::from_min_max(body.min,Pos2::new(split-20.0,body.bottom()));let detail=Rect::from_min_max(Pos2::new(split+20.0,body.top()+2.0),body.max);
                ui.painter().line_segment([Pos2::new(split,body.top()),Pos2::new(split,body.bottom())],Stroke::new(1.0,LINE));
                ui.scope_builder(egui::UiBuilder::new().max_rect(list),|ui|{ui.set_clip_rect(list);self.apps(ui);});
                ui.scope_builder(egui::UiBuilder::new().max_rect(detail),|ui|{ui.set_clip_rect(detail);self.detail(ui);});
            }
        });
        if let Some(viewer) = &mut self.log_viewer {
            let mut open = true;
            let mut refresh = false;
            let previous = viewer.selected;
            let size = ctx.content_rect().size();
            egui::Window::new("Local logs")
                .open(&mut open)
                .default_size([size.x.min(900.0) - 60.0, size.y.min(650.0) - 80.0])
                .show(&ctx, |ui| {
                    ui.label("Private diagnostic files on this PC. Review before sharing.");
                    ui.horizontal_wrapped(|ui| {
                        egui::ComboBox::from_id_salt("diagnostic-file")
                            .width(460.0)
                            .selected_text(
                                viewer
                                    .files
                                    .get(viewer.selected)
                                    .map(String::as_str)
                                    .unwrap_or("No log files"),
                            )
                            .show_ui(ui, |ui| {
                                for (index, name) in viewer.files.iter().enumerate() {
                                    ui.selectable_value(&mut viewer.selected, index, name);
                                }
                            });
                        refresh = ui.button("Refresh logs").clicked();
                        if ui.button("Copy folder path").clicked() {
                            ui.ctx().copy_text(self.directory.clone());
                        }
                    });
                    if viewer.selected != previous
                        && let Some(file) = viewer.files.get(viewer.selected)
                    {
                        viewer.text = if self.demo {
                            "Synthetic log preview. No private file was read.".into()
                        } else {
                            crate::diagnostics::read_log(
                                std::path::Path::new(&self.directory),
                                file,
                            )
                            .unwrap_or_else(|e| format!("Cannot read log: {e}"))
                        };
                    }
                    egui::ScrollArea::both()
                        .max_height((size.y - 250.0).max(180.0))
                        .show(ui, |ui| {
                            let mut text = viewer.text.as_str();
                            ui.add(
                                egui::TextEdit::multiline(&mut text)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY),
                            );
                        });
                });
            if !open {
                self.log_viewer = None;
            } else if refresh {
                self.show_logs();
            }
        }
        if let Some(editor) = &mut self.source_editor {
            let mut save = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("custom-source-editor")).show(&ctx,|ui|{
                ui.set_width(540.0);ui.heading(if editor.id.is_some(){"Edit your catalogue entry"}else{"Track a custom app"});
                ui.label("Saved only on this PC. Add a source you trust and the steps you use to update it.");
                let height=ctx.input(|i|i.viewport().inner_rect.map(|r|r.height()).unwrap_or(650.0));
                egui::ScrollArea::vertical().max_height((height-210.0).max(180.0)).show(ui,|ui|{
                ui.label("App name");ui.add_enabled(editor.manual,egui::TextEdit::singleline(&mut editor.name).desired_width(f32::INFINITY));
                if editor.manual {ui.label("Installed version (optional, entered by you)");ui.add(egui::TextEdit::singleline(&mut editor.source.manual_version).desired_width(f32::INFINITY));}
                ui.label("Official website or download page");ui.add(egui::TextEdit::singleline(&mut editor.source.url).hint_text("https://example.org/downloads").desired_width(f32::INFINITY));
                ui.label("GitHub release watch (optional)");ui.add(egui::TextEdit::singleline(&mut editor.source.repository).hint_text("GitHub project URL or owner/repository").desired_width(f32::INFINITY));
                ui.label("How to update / your notes");ui.add(egui::TextEdit::multiline(&mut editor.source.instructions).desired_rows(4).desired_width(f32::INFINITY));
                ui.label("Sources open in your browser. Notes are never executed as commands.");
                });
                if let Some(e)=&editor.error {ui.colored_label(Color32::DARK_RED,e);}
                ui.horizontal(|ui|{let button=ui.button("Save to my catalogue");self.editor_save_rect=Some(button.rect);save=button.clicked();cancel=ui.button("Cancel").clicked();});
            });
            if save {
                let validation = if editor.manual
                    && (editor.name.trim().is_empty() || editor.name.len() > 160)
                {
                    Err("Enter an app name (up to 160 characters).".into())
                } else {
                    crate::custom::save(
                        &mut App {
                            kind: if editor.manual {
                                "manual".into()
                            } else {
                                String::new()
                            },
                            ..Default::default()
                        },
                        editor.source.clone(),
                    )
                };
                if let Err(e) = validation {
                    editor.error = Some(e);
                } else {
                    let command = Command::Custom(
                        editor.id.clone(),
                        editor.name.clone(),
                        editor.source.clone(),
                    );
                    self.source_editor = None;
                    self.send(command, "Saving your private catalogue entry...");
                }
            } else if cancel {
                self.source_editor = None;
            }
        }
        if let Some(preview) = self.store_plan.clone() {
            let mut confirm = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("store-preview")).show(&ctx,|ui| {
                ui.set_width(520.0);ui.heading("Update Microsoft Store apps?");
                ui.label(format!("{} updates were reported at the last check.",preview.updates.len()));
                ui.label("AppTide rechecks this list first. Microsoft Store then updates its current pending set, which can include dependencies or newly available updates. AppTide pause settings do not apply to this batch.");
                ui.label("Save your work in Store apps first. Once started, Store controls the operation; AppTide cannot cancel an in-progress Store batch.");
                egui::ScrollArea::vertical().max_height(200.0).show(ui,|ui|{for a in &preview.updates{ui.label(format!("{} · {}",a.name,a.publisher));}});
                ui.horizontal(|ui|{let button=ui.button("Confirm Store updates");self.store_confirm_rect=Some(button.rect);confirm=button.clicked();cancel=ui.button("Cancel").clicked();});
            });
            if cancel {
                self.store_plan = None;
            }
            if confirm {
                self.store_plan = None;
                self.send(
                    Command::ApplyStore(preview),
                    "Microsoft Store is updating apps; wait for verification…",
                );
            }
        }
        if let Some(plan) = self.plan.clone() {
            let mut confirm = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("operation-preview")).show(&ctx,|ui|{ui.set_width(520.0);ui.heading(if plan.kind=="uninstall"{"Uninstall this app?"}else{"Review updates"});ui.label(if self.demo{"Demonstration only. No installed software will change."}else{"AppTide checks each reviewed target. A failed app is recorded and other apps continue. Cancellation, reboot requirements or failed verification scans stop the queue. Windows may ask for administrator permission."});egui::ScrollArea::vertical().max_height(280.0).show(ui,|ui|{for a in &plan.apps{ui.label(RichText::new(&a.name).strong());ui.label(format!("{}  ·  {}{}",a.winget_id.as_deref().unwrap_or(""),a.version,if plan.kind=="update"{format!(" → {}",a.available)}else{String::new()}));ui.separator();}});ui.horizontal(|ui|{let button=ui.button(if plan.kind=="update"{"Confirm updates"}else{"Confirm uninstall"});self.confirm_rect=Some(button.rect);confirm=button.clicked();cancel=ui.button("Cancel").clicked();});});
            if cancel {
                self.plan = None;
            }
            if confirm {
                self.plan = None;
                self.send(Command::Execute(plan), "Checking operation…");
            }
        }
        self.frames += 1;
        if self.restarting && !self.busy {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.scrolled_demo {
            self.page = Page::Installed;
        }
        if self.screenshot.is_some()
            && !self.screenshot_requested
            && self.screenshot_started.elapsed() >= Duration::from_millis(700)
        {
            self.screenshot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        if let Some(path) = self.screenshot.clone() {
            ctx.request_repaint_after(Duration::from_millis(80));
            let images = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| {
                        if let egui::Event::Screenshot { image, .. } = e {
                            Some(image.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            });
            for image in images {
                let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                let saved = image::save_buffer(
                    &path,
                    &pixels,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                );
                if saved.is_ok() {
                    let evidence = serde_json::json!({"width":image.width(),"height":image.height(),"demo":self.demo,"scrolled":self.scrolled_demo,"updateButtonVisible":self.update_rect.is_some_and(|r|rect.contains_rect(r)),"uninstallButtonVisible":self.uninstall_rect.is_some_and(|r|rect.contains_rect(r))});
                    let _ = std::fs::write(
                        format!("{path}.json"),
                        serde_json::to_vec_pretty(&evidence).unwrap(),
                    );
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
impl eframe::App for Tide {
    fn ui(&mut self, ui: &mut Ui, _: &mut eframe::Frame) {
        self.draw(ui);
    }
}
fn field(ui: &mut Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).size(12.0).color(MUTED));
    ui.add(egui::Label::new(value).wrap());
    ui.add_space(8.0);
}
fn app_row(ui: &mut Ui, a: &App, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 66.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &a.name)
    });
    let p = ui.painter();
    if selected || response.hovered() || response.has_focus() {
        p.rect_filled(
            rect,
            5,
            if selected {
                Color32::from_rgb(224, 238, 240)
            } else {
                Color32::from_rgb(236, 242, 246)
            },
        );
    }
    if response.has_focus() {
        p.rect_stroke(
            rect.shrink(1.0),
            5,
            Stroke::new(1.5, TEAL),
            egui::StrokeKind::Inside,
        );
    }
    let icon = Rect::from_min_size(rect.min + Vec2::new(12.0, 13.0), Vec2::splat(38.0));
    p.rect_filled(icon, 8, Color32::from_rgb(233, 239, 243));
    p.text(
        icon.center(),
        egui::Align2::CENTER_CENTER,
        a.name.chars().next().unwrap_or('A'),
        FontId::proportional(18.0),
        TEAL,
    );
    let name_width = (rect.width() - 260.0).max(60.0);
    let name_rect = Rect::from_min_size(
        rect.min + Vec2::new(64.0, 11.0),
        Vec2::new(name_width, 24.0),
    );
    let clipped = p.with_clip_rect(name_rect);
    clipped.text(
        name_rect.min,
        egui::Align2::LEFT_TOP,
        &a.name,
        FontId::proportional(15.0),
        INK,
    );
    let sub = if a.publisher.is_empty() {
        a.winget_id.as_deref().unwrap_or("Windows inventory")
    } else {
        &a.publisher
    };
    p.with_clip_rect(Rect::from_min_size(
        rect.min + Vec2::new(64.0, 36.0),
        Vec2::new(name_width, 20.0),
    ))
    .text(
        rect.min + Vec2::new(64.0, 36.0),
        egui::Align2::LEFT_TOP,
        sub,
        FontId::proportional(12.0),
        MUTED,
    );
    p.text(
        Pos2::new(rect.right() - 174.0, rect.top() + 13.0),
        egui::Align2::LEFT_TOP,
        &a.version,
        FontId::proportional(12.0),
        INK,
    );
    if !a.available.is_empty() {
        p.text(
            Pos2::new(rect.right() - 174.0, rect.top() + 36.0),
            egui::Align2::LEFT_TOP,
            format!("to {}", a.available),
            FontId::proportional(11.0),
            TEAL,
        );
    }
    let status = if !a.present {
        "Not present"
    } else if a.protected {
        "Protected"
    } else if a.ambiguous {
        "Ambiguous"
    } else if a.excluded {
        "Paused"
    } else if !a.available.is_empty() {
        "Update available"
    } else if a.winget_id.is_some() {
        "No listed update"
    } else if crate::releases::newer(a) {
        "Newer release"
    } else {
        crate::knowledge::route(a)
    };
    p.text(
        Pos2::new(rect.right() - 10.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        status,
        FontId::proportional(11.0),
        if eligible(a) { TEAL } else { MUTED },
    );
    p.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, LINE),
    );
    response.on_hover_text(&a.name)
}
fn open_store() {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe")
            .arg("ms-windows-store://downloadsandupdates")
            .spawn();
    }
}
fn open_settings() {
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("explorer.exe");
        c.arg("ms-settings:appsfeatures");
        let _ = c.spawn();
    }
}
fn open_url(url: &str) {
    if !url.starts_with("https://")
        || url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '"')
    {
        return;
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe").arg(url).spawn();
    }
}

fn open_project() {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe")
            .arg("https://github.com/GitKama/AppTide")
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn store_buttons_fit_and_confirmation_reaches_the_worker() {
        for size in [Vec2::new(1440.0, 900.0), Vec2::new(1040.0, 650.0)] {
            let ctx = egui::Context::default();
            let mut app = Tide::new(Service::open(Default::default(), true).unwrap(), &ctx);
            app.store_preview();
            for _ in 0..3 {
                frame(&mut app, &ctx, size, vec![]);
            }
            let rect = app.store_apply_rect.unwrap();
            assert!(Rect::from_min_size(Pos2::ZERO, size).contains_rect(rect));
            for pressed in [true, false] {
                let pos = rect.center();
                frame(
                    &mut app,
                    &ctx,
                    size,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                );
            }
            assert!(app.store_plan.is_some());
            assert!(app.catalog.receipts.is_empty());
            assert!(!app.busy);
            for _ in 0..3 {
                frame(&mut app, &ctx, size, vec![]);
            }
            let rect = app.store_confirm_rect.unwrap();
            assert!(Rect::from_min_size(Pos2::ZERO, size).contains_rect(rect));
            for pressed in [true, false] {
                let pos = rect.center();
                frame(
                    &mut app,
                    &ctx,
                    size,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                );
            }
            for _ in 0..100 {
                frame(&mut app, &ctx, size, vec![]);
                if !app.busy {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(app.store_plan.is_none());
            assert!(app.error.is_none());
            assert_eq!(app.catalog.receipts.len(), 1);
            assert_eq!(app.catalog.receipts[0]["kind"], "store update");
        }
    }
    fn frame(app: &mut Tide, ctx: &egui::Context, size: Vec2, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
    }
    #[test]
    fn long_list_keeps_actions_visible_at_both_window_sizes() {
        for size in [Vec2::new(1440.0, 900.0), Vec2::new(1040.0, 650.0)] {
            let ctx = egui::Context::default();
            let mut app = Tide::new(Service::open(Default::default(), true).unwrap(), &ctx);
            app.page = Page::Installed;
            for _ in 0..3 {
                frame(&mut app, &ctx, size, vec![]);
            }
            let before = app.update_rect.unwrap();
            app.scrolled_demo = true;
            for _ in 0..3 {
                frame(&mut app, &ctx, size, vec![]);
            }
            assert_eq!(app.update_rect.unwrap(), before);
            let viewport = Rect::from_min_size(Pos2::ZERO, size);
            assert!(viewport.contains_rect(app.update_rect.unwrap()));
            assert!(viewport.contains_rect(app.uninstall_rect.unwrap()));
        }
    }
    #[test]
    fn activity_log_button_reads_a_real_file_without_explorer() {
        let directory = std::env::temp_dir().join(format!(
            "apptide-log-viewer-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(directory.join("receipts")).unwrap();
        let path = directory.join("receipts/download.log");
        std::fs::write(&path, "HTTP 404: synthetic installer file missing").unwrap();
        let ctx = egui::Context::default();
        let mut app = Tide::new(Service::open(Default::default(), true).unwrap(), &ctx);
        // Synthetic worker; actual local log-reader code and UI click.
        app.demo = false;
        app.directory = directory.to_string_lossy().into_owned();
        app.page = Page::Activity;
        let size = Vec2::new(1040.0, 650.0);
        for _ in 0..3 {
            frame(&mut app, &ctx, size, vec![]);
        }
        let pos = app.logs_rect.unwrap().center();
        for pressed in [true, false] {
            frame(
                &mut app,
                &ctx,
                size,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        let viewer = app.log_viewer.as_ref().unwrap();
        assert_eq!(viewer.files, vec!["receipts/download.log"]);
        assert!(viewer.text.contains("HTTP 404"));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "HTTP 404: synthetic installer file missing"
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory.join("receipts")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn update_all_confirmation_runs_three_apps_and_keeps_summary() {
        let ctx = egui::Context::default();
        let mut app = Tide::new(Service::open(Default::default(), true).unwrap(), &ctx);
        let size = Vec2::new(1040.0, 650.0);
        for confirm in [false, true] {
            for _ in 0..3 {
                frame(&mut app, &ctx, size, vec![]);
            }
            let rect = if confirm {
                app.confirm_rect
            } else {
                app.update_all_rect
            }
            .unwrap();
            assert!(Rect::from_min_size(Pos2::ZERO, size).contains_rect(rect));
            let pos = rect.center();
            for pressed in [true, false] {
                frame(
                    &mut app,
                    &ctx,
                    size,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                );
            }
            if !confirm {
                assert_eq!(app.plan.as_ref().unwrap().apps.len(), 3);
            }
        }
        for _ in 0..100 {
            frame(&mut app, &ctx, size, vec![]);
            if !app.busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!app.busy);
        assert!(app.error.is_none());
        assert_eq!(app.catalog.receipts.len(), 3);
        assert!(
            app.catalog
                .receipts
                .iter()
                .all(|r| r["status"] == "verified")
        );
        assert_eq!(app.catalog.extra["lastBatch"]["status"], "complete");
        assert!(
            app.catalog.extra["lastBatch"]["summary"]
                .as_str()
                .unwrap()
                .contains("3 verified")
        );
    }
    #[test]
    fn clicking_update_opens_preview_without_executing() {
        let ctx = egui::Context::default();
        let mut app = Tide::new(Service::open(Default::default(), true).unwrap(), &ctx);
        let size = Vec2::new(1440.0, 900.0);
        for _ in 0..3 {
            frame(&mut app, &ctx, size, vec![]);
        }
        let pos = app.update_rect.unwrap().center();
        frame(
            &mut app,
            &ctx,
            size,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        frame(
            &mut app,
            &ctx,
            size,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert!(app.plan.is_some());
        assert!(!app.busy);
        assert!(app.catalog.receipts.is_empty());
    }
    #[test]
    fn custom_route_stays_visible_and_is_never_a_winget_plan() {
        let ctx = egui::Context::default();
        let mut app = Tide::new(Service::open(Default::default(), true).unwrap(), &ctx);
        app.catalogue_preview();
        let size = Vec2::new(1040.0, 650.0);
        for _ in 0..3 {
            frame(&mut app, &ctx, size, vec![]);
        }
        assert!(Rect::from_min_size(Pos2::ZERO, size).contains_rect(app.update_rect.unwrap()));
        assert!(crate::service::prepare(&app.catalog, "update", &["recipe-demo".into()]).is_err());
        assert!(app.catalog.apps["recipe-demo"].knowledge.is_some());
    }
    #[test]
    fn compact_catalogue_editor_saves_through_the_worker() {
        let ctx = egui::Context::default();
        let mut app = Tide::new(Service::open(Default::default(), true).unwrap(), &ctx);
        app.editor_preview();
        let size = Vec2::new(1040.0, 650.0);
        for _ in 0..3 {
            frame(&mut app, &ctx, size, vec![]);
        }
        let rect = app.editor_save_rect.unwrap();
        assert!(Rect::from_min_size(Pos2::ZERO, size).contains_rect(rect));
        let pos = rect.center();
        for pressed in [true, false] {
            frame(
                &mut app,
                &ctx,
                size,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        for _ in 0..100 {
            frame(&mut app, &ctx, size, vec![]);
            if !app.busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(app.source_editor.is_none());
        assert!(app.error.is_none());
        let a = app
            .catalog
            .apps
            .values()
            .find(|a| a.name == "Orchid utility")
            .unwrap();
        assert_eq!(a.version, "1.2.0");
        assert_eq!(
            crate::custom::get(a).unwrap().url,
            "https://example.org/downloads"
        );
        assert!(!actionable(a));
    }
}
