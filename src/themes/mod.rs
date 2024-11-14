mod classic;
mod dark;
mod light;

#[derive(Default, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Theme {
    #[default]
    Dark,
    Light,
    Classic,
}

pub struct Colors {
    bg: egui::Color32,
    surface: egui::Color32,
    border: egui::Color32,
    text: egui::Color32,
    accent: egui::Color32,
}

impl Theme {
    pub fn style(self) -> egui::Style {
        let mut style = style();

        style.visuals = match self {
            Theme::Dark => visuals(&dark::colors()),
            Theme::Light => visuals(&light::colors()),
            Theme::Classic => visuals(&classic::colors()),
        };

        style
    }
}

pub fn style() -> egui::Style {
    let mut style = egui::Style::default();

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(16.0, 8.0);
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));

    style
}

pub fn visuals(colors: &Colors) -> egui::Visuals {
    let mut visual = egui::Visuals::dark();

    let bg = colors.bg;
    let surface = colors.surface;
    let border = colors.border;
    let text = colors.text;
    let accent = colors.accent;

    visual.window_fill = bg;
    visual.panel_fill = surface;

    visual.selection.bg_fill = accent;
    visual.selection.stroke = egui::Stroke::new(1.0, bg);

    visual.widgets.noninteractive.bg_fill = bg;
    visual.widgets.noninteractive.weak_bg_fill = surface;
    visual.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text);
    visual.widgets.noninteractive.bg_stroke = egui::Stroke::new(2.0, border);

    visual.widgets.inactive.bg_fill = bg;
    visual.widgets.inactive.weak_bg_fill = surface;
    visual.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text);
    visual.widgets.inactive.bg_stroke = egui::Stroke::new(2.0, border);
    visual.widgets.inactive.rounding = egui::Rounding::same(18.0);

    visual.widgets.hovered.bg_fill = accent;
    visual.widgets.hovered.weak_bg_fill = accent;
    visual.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, bg);
    visual.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent);
    visual.widgets.hovered.rounding = egui::Rounding::same(18.0);

    visual.widgets.active.bg_fill = accent;
    visual.widgets.active.weak_bg_fill = accent;
    visual.widgets.active.fg_stroke = egui::Stroke::new(1.0, bg);
    visual.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent);
    visual.widgets.active.rounding = egui::Rounding::same(18.0);

    visual.widgets.open.bg_fill = accent;
    visual.widgets.open.weak_bg_fill = accent;
    visual.widgets.open.fg_stroke = egui::Stroke::new(1.0, bg);
    visual.widgets.open.bg_stroke = egui::Stroke::new(1.0, accent);
    visual.widgets.open.rounding = egui::Rounding::same(18.0);

    visual
}
