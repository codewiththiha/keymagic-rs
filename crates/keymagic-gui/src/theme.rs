//! Amber theme: palette values, spacing, fonts and egui styling.

use egui::{
    Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Style, Visuals,
};
use std::sync::Arc;

/// An sRGB colour as bytes.
pub type Rgb = [u8; 3];

/// The amber accent, #F5B53F.
pub const ACCENT: Rgb = [0xF5, 0xB5, 0x3F];
/// Spacing unit in points; gaps and paddings are multiples of it.
pub const SPACING: u8 = 8;
/// Corner radius of panels, windows and widgets, in points.
pub const ROUNDING: u8 = 8;

const INTER: &[u8] = include_bytes!("../assets/fonts/Inter-Variable.ttf");
const NOTO_SANS_MYANMAR: &[u8] = include_bytes!("../assets/fonts/NotoSansMyanmar-Variable.ttf");
const JETBRAINS_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Variable.ttf");

/// Light or dark appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Dark,
    Light,
}

/// The colours one variant paints with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub background: Rgb,
    pub surface: Rgb,
    pub text: Rgb,
    pub muted: Rgb,
    pub accent: Rgb,
    pub on_accent: Rgb,
}

impl Variant {
    /// The other appearance.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    /// The button label that switches to the other appearance.
    #[must_use]
    pub const fn switch_label(self) -> &'static str {
        match self {
            Self::Dark => "Light mode",
            Self::Light => "Dark mode",
        }
    }

    /// The colours for this appearance.
    #[must_use]
    pub const fn palette(self) -> Palette {
        match self {
            Self::Dark => Palette {
                background: [0x1B, 0x1A, 0x17],
                surface: [0x26, 0x24, 0x1F],
                text: [0xF2, 0xEE, 0xE6],
                muted: [0xB4, 0xAE, 0xA2],
                accent: ACCENT,
                on_accent: [0x1B, 0x1A, 0x17],
            },
            Self::Light => Palette {
                background: [0xFB, 0xF8, 0xF2],
                surface: [0xFF, 0xFF, 0xFF],
                text: [0x1F, 0x1B, 0x14],
                muted: [0x6B, 0x64, 0x57],
                accent: ACCENT,
                on_accent: [0x1F, 0x1B, 0x14],
            },
        }
    }

    const fn preference(self) -> egui::ThemePreference {
        match self {
            Self::Dark => egui::ThemePreference::Dark,
            Self::Light => egui::ThemePreference::Light,
        }
    }
}

/// WCAG contrast ratio between two colours, from 1 to 21.
#[must_use]
pub fn contrast(a: Rgb, b: Rgb) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

fn luminance(c: Rgb) -> f32 {
    let linear = |byte: u8| {
        let value = f32::from(byte) / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.0722_f32.mul_add(
        linear(c[2]),
        0.7152_f32.mul_add(linear(c[1]), 0.2126 * linear(c[0])),
    )
}

/// Loads the bundled fonts and styles for both appearances.
pub fn install(ctx: &Context) {
    ctx.set_fonts(fonts());
    ctx.set_visuals_of(egui::Theme::Dark, visuals(Variant::Dark));
    ctx.set_visuals_of(egui::Theme::Light, visuals(Variant::Light));
    let spacing = f32::from(SPACING);
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(spacing, spacing);
    });
    select(ctx, Variant::Dark);
}

/// The accent as an egui colour.
#[must_use]
pub const fn accent_color() -> Color32 {
    color(ACCENT)
}

/// Sets how long egui animates; zero when motion is reduced.
pub fn set_reduced_motion(ctx: &Context, reduce: bool) {
    let time = if reduce {
        0.0
    } else {
        Style::default().animation_time
    };
    ctx.all_styles_mut(|style| style.animation_time = time);
}

/// Switches the whole interface to `variant`.
pub fn select(ctx: &Context, variant: Variant) {
    ctx.set_theme(variant.preference());
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::empty();
    for (name, bytes) in [
        ("inter", INTER),
        ("noto-sans-myanmar", NOTO_SANS_MYANMAR),
        ("jetbrains-mono", JETBRAINS_MONO),
    ] {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    let proportional = vec!["inter".to_owned(), "noto-sans-myanmar".to_owned()];
    let monospace = vec!["jetbrains-mono".to_owned(), "noto-sans-myanmar".to_owned()];
    fonts
        .families
        .insert(FontFamily::Proportional, proportional);
    fonts.families.insert(FontFamily::Monospace, monospace);
    fonts
}

fn visuals(variant: Variant) -> Visuals {
    let palette = variant.palette();
    let mut visuals = match variant {
        Variant::Dark => Visuals::dark(),
        Variant::Light => Visuals::light(),
    };
    let rounding = CornerRadius::same(ROUNDING);
    let accent = color(palette.accent);
    let on_accent = color(palette.on_accent);
    visuals.override_text_color = Some(color(palette.text));
    visuals.panel_fill = color(palette.background);
    visuals.window_fill = color(palette.surface);
    visuals.extreme_bg_color = color(palette.surface);
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = Stroke::new(1.0, on_accent);
    visuals.window_corner_radius = rounding;
    visuals.menu_corner_radius = rounding;
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = rounding;
    }
    visuals.widgets.hovered.bg_fill = accent;
    visuals.widgets.hovered.weak_bg_fill = accent;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, on_accent);
    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.active.weak_bg_fill = accent;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, on_accent);
    visuals
}

const fn color(rgb: Rgb) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

#[cfg(test)]
mod tests {
    use super::{ACCENT, ROUNDING, SPACING, Variant, contrast, set_reduced_motion};

    #[test]
    fn accent_is_the_amber_from_the_design() {
        assert_eq!(ACCENT, [0xF5, 0xB5, 0x3F]);
        assert_eq!(Variant::Dark.palette().accent, ACCENT);
        assert_eq!(Variant::Light.palette().accent, ACCENT);
    }

    #[test]
    fn body_text_meets_aa_contrast_in_both_variants() {
        for variant in [Variant::Dark, Variant::Light] {
            let palette = variant.palette();
            assert!(contrast(palette.text, palette.background) >= 4.5);
            assert!(contrast(palette.text, palette.surface) >= 4.5);
            assert!(contrast(palette.muted, palette.background) >= 4.5);
        }
    }

    #[test]
    fn text_on_the_accent_meets_aa_contrast() {
        for variant in [Variant::Dark, Variant::Light] {
            let palette = variant.palette();
            assert!(contrast(palette.on_accent, palette.accent) >= 4.5);
        }
    }

    #[test]
    fn contrast_is_symmetric_and_bounded() {
        let black = [0, 0, 0];
        let white = [255, 255, 255];
        assert!((contrast(black, white) - 21.0).abs() < 0.01);
        assert!((contrast(white, black) - contrast(black, white)).abs() < f32::EPSILON);
        assert!((contrast(white, white) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn variants_toggle_and_label_the_other_mode() {
        assert_eq!(Variant::Dark.toggled(), Variant::Light);
        assert_eq!(Variant::Light.toggled(), Variant::Dark);
        assert_eq!(Variant::Dark.switch_label(), "Light mode");
        assert_eq!(Variant::Light.switch_label(), "Dark mode");
    }

    #[test]
    fn reduced_motion_zeroes_the_animation_time() {
        let ctx = egui::Context::default();
        set_reduced_motion(&ctx, true);
        assert!(ctx.global_style().animation_time.abs() < f32::EPSILON);
        set_reduced_motion(&ctx, false);
        assert!(ctx.global_style().animation_time > 0.0);
    }

    #[test]
    fn spacing_and_rounding_follow_the_scale() {
        assert_eq!(SPACING, 8);
        assert_eq!(ROUNDING, 8);
    }
}
