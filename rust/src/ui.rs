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
    Execute(Plan),
    Pause(String, bool),
    Import,
    Watch(String, String),
    RefreshWatch(String),
    AutoPreference(Option<String>, bool),
    Automatic,
    Folder(String, bool),
}
enum Event {
    State(Box<Catalog>, Vec<String>, String),
    Done(std::result::Result<(), String>),
}
#[derive(Clone, Copy, PartialEq)]
enum Page {
    Updates,
    Installed,
    Catalogue,
    Activity,
    Settings,
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
    demo: bool,
    directory: String,
    details_reset: bool,
    watch_input: String,
    watch_app: String,
    folder_input: String,
    coverage_filter: usize,
    pub screenshot: Option<String>,
    frames: u32,
    pub scrolled_demo: bool,
    pub update_rect: Option<Rect>,
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
        let cancel = service.cancel.clone();
        let (tx, commands) = mpsc::channel();
        let (events, rx) = mpsc::channel();
        let context = ctx.clone();
        std::thread::spawn(move || {
            let mut service = service;
            while let Ok(command) = commands.recv() {
                let result = match command {
                    Command::Scan => {
                        service.cancel.store(false, Ordering::Relaxed);
                        service.scan().map(|_| ())
                    }
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
                    Command::Automatic => service.automatic(|phase, cat| {
                        let _ =
                            events.send(Event::State(Box::new(cat.clone()), vec![], phase.into()));
                        context.request_repaint();
                    }),
                };
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
            error: None,
            plan: None,
            demo,
            directory,
            details_reset: false,
            watch_input: String::new(),
            watch_app: String::new(),
            folder_input: String::new(),
            coverage_filter: 0,
            screenshot: None,
            frames: 0,
            scrolled_demo: false,
            update_rect: None,
            uninstall_rect: None,
            list_rect: None,
        }
    }
    fn send(&mut self, command: Command, phase: &str) {
        if self.busy {
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
        ui.add(egui::Label::new(RichText::new(&a.publisher).color(MUTED)).truncate())
            .on_hover_text(&a.publisher);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Installed").color(MUTED));
            ui.label(RichText::new(&a.version).strong());
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
            && (crate::releases::repository(&a).is_some() || a.knowledge.is_some());
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
                    && (self.show_system || !a.protected)
                    && (self.page != Page::Updates
                        || !a.available.is_empty()
                        || crate::releases::newer(a))
                    && (self.page == Page::Updates
                        || match self.coverage_filter {
                            1 => a.winget_id.is_none() && a.knowledge.is_some(),
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
                ui.separator();
            }
        });
    }
    fn settings(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui,|ui|{
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
                        self.error = Some(e);
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
            }
        }
        let ctx = ui.ctx().clone();
        if !self.demo && self.catalog.settings["automatic"] == true {
            ctx.request_repaint_after(Duration::from_secs(30));
            if !self.busy && self.plan.is_none() && crate::service::automatic_due(&self.catalog) {
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
            ui.horizontal(|ui|{ui.vertical(|ui|{ui.heading(RichText::new(match self.page{Page::Updates=>"Updates",Page::Installed=>"Installed apps",Page::Catalogue=>"My catalogue",Page::Activity=>"Activity",Page::Settings=>"Settings"}).size(30.0));ui.label(RichText::new("A little maintenance. Everything in one place.").color(MUTED));});
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{if self.page==Page::Updates{let ids:Vec<_>=self.catalog.apps.values().filter(|a|eligible(a)).map(|a|a.uid.clone()).collect();if ui.add_enabled(!self.busy&&!ids.is_empty(),egui::Button::new(RichText::new(format!("Update all ({})",ids.len())).color(Color32::WHITE)).fill(TEAL)).clicked(){self.prepare("update",ids);}}
                    if ui.add_enabled(!self.busy,egui::Button::new("Check for updates")).clicked(){self.send(Command::Scan,"Reading installed software and WinGet sources…");}
                });
            });ui.add_space(18.0);
            if self.demo{ui.colored_label(TEAL,"DEMO  ·  Synthetic apps. Update and uninstall actions are simulated.");}
            if let Some(error)=&self.error{ui.colored_label(Color32::from_rgb(161,43,42),error);}
            for warning in &self.warnings{ui.colored_label(Color32::from_rgb(137,80,14),RichText::new(warning).size(12.0));}
            if self.busy {ui.horizontal(|ui|{ui.spinner();ui.label(&self.phase);if ui.button("Stop queue").clicked(){self.cancel.store(true,Ordering::Relaxed);}});}
            let learned=self.catalog.apps.values().filter(|a|a.present && a.winget_id.is_none() && a.knowledge.is_some()).count();let present=self.catalog.apps.values().filter(|a|a.present).count();let matched=self.catalog.apps.values().filter(|a|a.present&&a.winget_id.is_some()).count();
            ui.horizontal(|ui|{ui.label(RichText::new(format!("{present} app records  ·  {matched} WinGet / {learned} other known routes  ·  {} catalogue scans",self.catalog.scans)).size(12.0).color(MUTED));});ui.separator();ui.add_space(10.0);
            if matches!(self.page,Page::Activity){self.activity(ui);}else if self.page==Page::Settings{self.settings(ui);}else{
                let body=ui.available_rect_before_wrap();let details_width=290.0;let split=body.right()-details_width-20.0;
                let list=Rect::from_min_max(body.min,Pos2::new(split-20.0,body.bottom()));let detail=Rect::from_min_max(Pos2::new(split+20.0,body.top()+2.0),body.max);
                ui.painter().line_segment([Pos2::new(split,body.top()),Pos2::new(split,body.bottom())],Stroke::new(1.0,LINE));
                ui.scope_builder(egui::UiBuilder::new().max_rect(list),|ui|{ui.set_clip_rect(list);self.apps(ui);});
                ui.scope_builder(egui::UiBuilder::new().max_rect(detail),|ui|{ui.set_clip_rect(detail);self.detail(ui);});
            }
        });
        if let Some(plan) = self.plan.clone() {
            let mut confirm = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("operation-preview")).show(&ctx,|ui|{ui.set_width(520.0);ui.heading(if plan.kind=="uninstall"{"Uninstall this app?"}else{"Review updates"});ui.label(if self.demo{"Demonstration only. No installed software will change."}else{"AppTide will check the installed versions again before running WinGet. Windows may ask for administrator permission."});egui::ScrollArea::vertical().max_height(280.0).show(ui,|ui|{for a in &plan.apps{ui.label(RichText::new(&a.name).strong());ui.label(format!("{}  ·  {}{}",a.winget_id.as_deref().unwrap_or(""),a.version,if plan.kind=="update"{format!(" → {}",a.available)}else{String::new()}));ui.separator();}});ui.horizontal(|ui|{confirm=ui.button(if plan.kind=="update"{"Confirm updates"}else{"Confirm uninstall"}).clicked();cancel=ui.button("Cancel").clicked();});});
            if cancel {
                self.plan = None;
            }
            if confirm {
                self.plan = None;
                self.send(Command::Execute(plan), "Checking operation…");
            }
        }
        self.frames += 1;
        if self.scrolled_demo {
            self.page = Page::Installed;
        }
        if self.screenshot.is_some() && self.frames == 5 {
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
}
