use eframe::egui::{self, Color32, FontId, Stroke, Vec2};

pub(super) struct Palette {
    pub(super) canvas: Color32,
    pub(super) sidebar: Color32,
    pub(super) text: Color32,
    pub(super) muted: Color32,
    pub(super) accent: Color32,
    pub(super) on_accent: Color32,
    pub(super) line: Color32,
    pub(super) control: Color32,
    pub(super) hover: Color32,
    pub(super) selected: Color32,
    pub(super) icon: Color32,
    pub(super) warning: Color32,
    pub(super) danger: Color32,
}

impl Palette {
    const DARK: Self = Self {
        canvas: Color32::from_rgb(22, 29, 37),
        sidebar: Color32::from_rgb(29, 40, 51),
        text: Color32::from_rgb(230, 237, 243),
        muted: Color32::from_rgb(176, 192, 208),
        accent: Color32::from_rgb(118, 218, 210),
        on_accent: Color32::from_rgb(16, 36, 33),
        line: Color32::from_rgb(81, 99, 116),
        control: Color32::from_rgb(40, 53, 66),
        hover: Color32::from_rgb(43, 60, 73),
        selected: Color32::from_rgb(35, 68, 73),
        icon: Color32::from_rgb(50, 68, 82),
        warning: Color32::from_rgb(243, 188, 121),
        danger: Color32::from_rgb(255, 155, 158),
    };
    const LIGHT: Self = Self {
        canvas: Color32::from_rgb(247, 249, 251),
        sidebar: Color32::from_rgb(234, 240, 245),
        text: Color32::from_rgb(36, 53, 74),
        muted: Color32::from_rgb(82, 102, 124),
        accent: Color32::from_rgb(7, 109, 114),
        on_accent: Color32::from_rgb(255, 255, 255),
        line: Color32::from_rgb(179, 195, 206),
        control: Color32::from_rgb(255, 255, 255),
        hover: Color32::from_rgb(236, 242, 246),
        selected: Color32::from_rgb(211, 230, 232),
        icon: Color32::from_rgb(233, 239, 243),
        warning: Color32::from_rgb(137, 80, 14),
        danger: Color32::from_rgb(163, 42, 52),
    };
    pub(super) fn for_mode(dark: bool) -> &'static Self {
        if dark { &Self::DARK } else { &Self::LIGHT }
    }
}

pub(super) fn dark_mode(settings: &serde_json::Value) -> bool {
    settings["darkMode"].as_bool().unwrap_or(true)
}

pub(super) fn apply(ctx: &egui::Context, dark: bool) {
    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(theme);
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(if dark {
        egui::SystemTheme::Dark
    } else {
        egui::SystemTheme::Light
    }));
    let colors = Palette::for_mode(dark);
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.override_text_color = Some(colors.text);
    visuals.panel_fill = colors.canvas;
    visuals.window_fill = colors.canvas;
    visuals.extreme_bg_color = colors.control;
    visuals.selection.bg_fill = colors.selected;
    visuals.selection.stroke = Stroke::new(1.0, colors.accent);
    visuals.widgets.inactive.bg_fill = colors.control;
    visuals.widgets.inactive.weak_bg_fill = colors.control;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, colors.line);
    visuals.widgets.hovered.bg_fill = colors.hover;
    visuals.widgets.hovered.weak_bg_fill = colors.hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, colors.accent);
    visuals.widgets.active.bg_fill = colors.selected;
    visuals.widgets.active.weak_bg_fill = colors.selected;
    visuals.hyperlink_color = colors.accent;
    visuals.weak_text_color = Some(colors.muted);
    visuals.warn_fg_color = colors.warning;
    visuals.error_fg_color = colors.danger;
    ctx.set_visuals(visuals);
    ctx.style_mut_of(theme, |s| {
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
}
