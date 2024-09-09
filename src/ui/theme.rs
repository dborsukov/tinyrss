use eframe::{
    egui::{
        style::{Selection, WidgetVisuals, Widgets},
        Color32, Rounding, Stroke, Visuals,
    },
    epaint::Shadow,
};

pub struct Spacing {
    pub large: f32,
    pub medium: f32,
    pub small: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            large: 10.0,
            medium: 5.0,
            small: 2.0,
        }
    }
}

pub struct RoundingVar {
    pub medium: Rounding,
    pub large: Rounding,
}

impl Default for RoundingVar {
    fn default() -> Self {
        Self {
            medium: Rounding::same(2.0),
            large: Rounding::same(4.0),
        }
    }
}

pub struct Colors {
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub bg: Color32,
    pub bg_darker: Color32,
    pub bg_darkest: Color32,
    pub warning: Color32,
}

impl Colors {
    pub fn dark() -> Self {
        Self {
            text: Color32::from_rgb(235, 232, 224),
            text_dim: Color32::from_white_alpha(30),
            accent: Color32::from_rgb(162, 123, 92),
            bg: Color32::from_rgb(63, 78, 79),
            bg_darker: Color32::from_rgb(44, 54, 57),
            bg_darkest: Color32::from_rgb(18, 22, 23),
            warning: Color32::from_rgb(183, 62, 62),
        }
    }
}

pub struct Theme {
    pub colors: Colors,
    pub visuals: Visuals,
    pub spacing: Spacing,
    pub rounding: RoundingVar,
}

impl Theme {
    pub fn from_colors(colors: Colors) -> Self {
        let spacing = Spacing::default();
        let rounding = RoundingVar::default();

        let widgets = Widgets {
            noninteractive: WidgetVisuals {
                bg_fill: colors.bg_darker,
                weak_bg_fill: colors.bg,
                bg_stroke: Stroke::new(1.0, colors.bg),
                fg_stroke: Stroke::new(1.0, colors.text),
                rounding: rounding.medium,
                expansion: 0.0,
            },
            inactive: WidgetVisuals {
                bg_fill: colors.bg,
                weak_bg_fill: colors.bg,
                bg_stroke: Stroke::default(),
                fg_stroke: Stroke::new(1.0, colors.text),
                rounding: rounding.medium,
                expansion: 0.0,
            },
            hovered: WidgetVisuals {
                bg_fill: Color32::from_gray(70),
                weak_bg_fill: colors.bg,
                bg_stroke: Stroke::new(1.0, Color32::from_gray(150)),
                fg_stroke: Stroke::new(1.5, Color32::from_gray(240)),
                rounding: rounding.medium,
                expansion: 0.0,
            },
            active: WidgetVisuals {
                bg_fill: Color32::from_gray(55),
                weak_bg_fill: colors.bg,
                bg_stroke: Stroke::new(1.0, Color32::WHITE),
                fg_stroke: Stroke::new(2.0, Color32::WHITE),
                rounding: rounding.medium,
                expansion: 0.0,
            },
            open: WidgetVisuals {
                bg_fill: Color32::from_gray(27),
                weak_bg_fill: colors.bg,
                bg_stroke: Stroke::new(1.0, Color32::from_gray(60)),
                fg_stroke: Stroke::new(1.0, Color32::from_gray(210)),
                rounding: rounding.medium,
                expansion: 0.0,
            },
        };

        let selection = Selection {
            bg_fill: colors.accent,
            stroke: Stroke::new(1.0, colors.text),
        };

        let visuals = Visuals {
            widgets,
            selection,

            panel_fill: colors.bg_darker,
            window_fill: colors.bg_darker,
            hyperlink_color: colors.accent,
            extreme_bg_color: colors.bg_darkest,

            window_shadow: Shadow {
                extrusion: 16.0,
                color: Color32::from_black_alpha(40),
            },

            menu_rounding: rounding.medium,
            ..Visuals::default()
        };

        Self {
            colors,
            visuals,
            spacing,
            rounding,
        }
    }
}
