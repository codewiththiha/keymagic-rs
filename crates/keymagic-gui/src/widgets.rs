//! Shared widgets: cards, buttons, switches, rows and key caps.

use crate::nav::Page;
use crate::theme::{self, CONTROL_ROUNDING, ROUNDING};
use egui::{
    Align2, Button, Color32, CornerRadius, CursorIcon, FontId, Frame, Margin, Painter, Pos2, Rect,
    Response, RichText, Role, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetText,
};

const SWITCH_WIDTH: f32 = 36.0;
const SWITCH_HEIGHT: f32 = 20.0;
const SWITCH_RADIUS: u8 = 10;
const KEY_GAP: f32 = 4.0;
const SETTING_HEIGHT: f32 = 52.0;
const NAV_ITEM_HEIGHT: f32 = 36.0;

/// Secondary text, such as a note under a setting.
#[must_use]
pub fn muted(ui: &Ui, text: &str) -> RichText {
    RichText::new(text).color(theme::muted_color(ui.visuals()))
}

/// A surface with 16 points of padding and the card radius.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(ui.visuals().window_fill)
        .corner_radius(CornerRadius::same(ROUNDING))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// A group title above a card, with the gap that separates groups.
pub fn section<R>(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.label(RichText::new(title).strong());
    ui.add_space(8.0);
    let inner = card(ui, add);
    ui.add_space(24.0);
    inner
}

/// The one amber button on a view: the primary action.
pub fn primary(ui: &mut Ui, text: &str) -> Response {
    let response = ui.add(
        Button::new(RichText::new(text).color(theme::color(theme::ON_ACCENT)))
            .fill(theme::accent_color()),
    );
    focus_ring(ui, &response, CONTROL_ROUNDING);
    response
}

/// A secondary button. Its focus ring is drawn here, since egui draws none.
pub fn button(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    let response = ui.add(Button::new(text));
    focus_ring(ui, &response, CONTROL_ROUNDING);
    response
}

/// Paints the keyboard focus ring round a focused control.
pub fn focus_ring(ui: &Ui, response: &Response, radius: impl Into<CornerRadius>) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(3.0),
            radius,
            Stroke::new(2.0, theme::accent_color()),
            StrokeKind::Outside,
        );
    }
}

/// A switch with no text. Its accessible name is `label`.
pub fn switch(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(SWITCH_WIDTH, SWITCH_HEIGHT), Sense::click());
    toggle_on_click(&mut response, on, label);
    paint_switch(ui, rect, *on);
    focus_ring(ui, &response, SWITCH_RADIUS);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A sidebar entry filling the rail width. The selected entry is filled.
pub fn nav_item(ui: &mut Ui, page: Page, selected: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, NAV_ITEM_HEIGHT), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Tab, true, selected, page.label()));
    let visuals = ui.visuals();
    let fill = if selected {
        visuals.widgets.hovered.weak_bg_fill
    } else if response.hovered() {
        visuals.widgets.inactive.weak_bg_fill
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(CONTROL_ROUNDING), fill);
    let color = visuals.text_color();
    let icon_center = Pos2::new(rect.left() + 22.0, rect.center().y);
    draw_nav_icon(painter, icon_center, page, color);
    painter.text(
        Pos2::new(rect.left() + 42.0, rect.center().y),
        Align2::LEFT_CENTER,
        page.label(),
        FontId::proportional(14.0),
        color,
    );
    focus_ring(ui, &response, CONTROL_ROUNDING);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// Draws a nav entry's glyph from simple strokes; no icon font is needed.
fn draw_nav_icon(painter: &Painter, center: Pos2, page: Page, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let at = |dx: f32, dy: f32| Pos2::new(center.x + dx, center.y + dy);
    match page {
        Page::Keyboards => {
            let body = Rect::from_center_size(center, Vec2::new(16.0, 11.0));
            painter.rect_stroke(body, CornerRadius::same(3), stroke, StrokeKind::Inside);
            for dx in [-4.0, 0.0, 4.0] {
                painter.circle_filled(at(dx, -1.5), 1.0, color);
            }
            painter.line_segment([at(-3.0, 2.5), at(3.0, 2.5)], stroke);
        }
        Page::Tester => {
            painter.line_segment([at(0.0, -6.0), at(0.0, 6.0)], stroke);
            painter.line_segment([at(-3.0, -6.0), at(3.0, -6.0)], stroke);
            painter.line_segment([at(-3.0, 6.0), at(3.0, 6.0)], stroke);
        }
        Page::Editor => {
            painter.line_segment([at(-6.0, 6.0), at(5.0, -5.0)], Stroke::new(2.5, color));
            painter.line_segment([at(-6.0, 6.0), at(-4.0, 4.0)], stroke);
        }
        Page::Settings => {
            painter.circle_stroke(center, 4.5, stroke);
            for (dx, dy) in [(0.0, -7.0), (0.0, 7.0), (-7.0, 0.0), (7.0, 0.0)] {
                painter.line_segment([at(dx * 0.7, dy * 0.7), at(dx, dy)], stroke);
            }
        }
        Page::Hotkeys => {
            let cap = Rect::from_center_size(center, Vec2::new(14.0, 12.0));
            painter.rect_stroke(cap, CornerRadius::same(3), stroke, StrokeKind::Inside);
            painter.line_segment([at(-4.0, 2.5), at(4.0, 2.5)], stroke);
        }
        Page::About => {
            painter.circle_stroke(center, 7.0, stroke);
            painter.circle_filled(at(0.0, -3.5), 1.0, color);
            painter.line_segment([at(0.0, -0.5), at(0.0, 4.0)], stroke);
        }
    }
}

/// A settings row: name and note on the left, a switch on the right.
pub fn setting(ui: &mut Ui, title: &str, note: &str, on: &mut bool) -> Response {
    let width = ui.available_width();
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(width, SETTING_HEIGHT), Sense::click());
    toggle_on_click(&mut response, on, title);
    let hovered = response.hovered();
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    let visuals = ui.visuals();
    let painter = ui.painter();
    if hovered {
        painter.rect_filled(
            rect.expand(4.0),
            CornerRadius::same(CONTROL_ROUNDING),
            visuals.widgets.hovered.weak_bg_fill,
        );
    }
    painter.text(
        Pos2::new(rect.left() + 4.0, rect.top() + 8.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(14.0),
        visuals.text_color(),
    );
    painter.text(
        Pos2::new(rect.left() + 4.0, rect.top() + 28.0),
        Align2::LEFT_TOP,
        note,
        FontId::proportional(12.0),
        theme::muted_color(visuals),
    );
    let track = Rect::from_center_size(
        Pos2::new(rect.right() - SWITCH_WIDTH / 2.0 - 4.0, rect.center().y),
        Vec2::new(SWITCH_WIDTH, SWITCH_HEIGHT),
    );
    paint_switch(ui, track, *on);
    focus_ring(ui, &response, CONTROL_ROUNDING);
    response
}

/// Flips `on` on click and reports the checkbox state under `label`.
fn toggle_on_click(response: &mut Response, on: &mut bool, label: &str) {
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(Role::CheckBox, true, *on, label));
}

/// Draws a switch track of `rect`, with its knob on the side for its state.
fn paint_switch(ui: &Ui, rect: Rect, on: bool) {
    let visuals = ui.visuals();
    let painter = ui.painter();
    let radius = rect.height() / 2.0;
    let (track, knob_x) = if on {
        (theme::accent_color(), rect.right() - radius)
    } else {
        (theme::muted_color(visuals), rect.left() + radius)
    };
    painter.rect_filled(rect, radius, track);
    painter.circle_filled(
        Pos2::new(knob_x, rect.center().y),
        radius - 4.0,
        theme::color(theme::ON_ACCENT),
    );
}

/// A framed shortcut label, such as `Ctrl+Shift+K`.
pub fn chip(ui: &mut Ui, text: &str) {
    Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(CornerRadius::same(CONTROL_ROUNDING))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            let label = ui.label(RichText::new(text).monospace());
            label.widget_info(|| WidgetInfo::labeled(Role::Label, true, format!("Hotkey {text}")));
        });
}

/// A keycap `width` keys wide and `unit` points per key, labelled with `label`.
pub fn key_cap(ui: &mut Ui, label: &str, width: f32, unit: f32) {
    let (slot, _) = ui.allocate_exact_size(Vec2::new(unit * width, unit), Sense::hover());
    let cap = Rect::from_min_size(
        slot.min,
        Vec2::new(unit.mul_add(width, -KEY_GAP), unit - KEY_GAP),
    );
    let visuals = ui.visuals();
    let painter = ui.painter();
    painter.rect_filled(
        cap,
        CornerRadius::same(CONTROL_ROUNDING),
        visuals.extreme_bg_color,
    );
    painter.text(
        cap.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::monospace(14.0),
        visuals.text_color(),
    );
}
