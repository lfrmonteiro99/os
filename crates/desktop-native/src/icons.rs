//! Category-specific app icons drawn with painter primitives.

use eframe::egui::{
    Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2,
};

/// Draw a category-specific icon inside the given rect.
/// Falls back to 2-letter abbreviation for unknown apps.
pub fn paint_app_icon(painter: &Painter, rect: Rect, name: &str, category: &str) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let white = Color32::WHITE;

    // Try known builtins first
    if paint_builtin_icon(painter, rect, name) {
        return;
    }

    // Category-specific generic icons
    match category {
        "Development" => paint_dev_icon(painter, rect),
        "Internet" => paint_globe_icon(painter, rect),
        "Games" => paint_game_icon(painter, rect),
        "Media" => paint_media_icon(painter, rect),
        "Communication" => paint_chat_icon(painter, rect),
        "Utilities" => paint_wrench_icon(painter, rect),
        "Productivity" => paint_doc_icon(painter, rect),
        "Graphics" => paint_palette_icon(painter, rect),
        "System" => paint_gear_icon(painter, rect),
        _ => {
            // Fallback: first 2 letters
            let symbol = if name.len() >= 2 { &name[..2] } else { name };
            painter.text(
                c,
                Align2::CENTER_CENTER,
                symbol,
                FontId::proportional(s * 0.35),
                white,
            );
        }
    }
}

fn paint_builtin_icon(painter: &Painter, rect: Rect, name: &str) -> bool {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let w = Color32::WHITE;
    let lw = (s * 0.04).max(1.0);

    match name.to_lowercase().as_str() {
        "terminal" => {
            // >_ prompt
            let inner = rect.shrink(s * 0.2);
            let mid_y = inner.center().y;
            painter.line_segment(
                [
                    Pos2::new(inner.left(), mid_y - s * 0.1),
                    Pos2::new(inner.center().x - s * 0.05, mid_y),
                ],
                Stroke::new(lw * 2.0, w),
            );
            painter.line_segment(
                [
                    Pos2::new(inner.left(), mid_y + s * 0.1),
                    Pos2::new(inner.center().x - s * 0.05, mid_y),
                ],
                Stroke::new(lw * 2.0, w),
            );
            painter.line_segment(
                [
                    Pos2::new(inner.center().x + s * 0.02, mid_y + s * 0.12),
                    Pos2::new(inner.right(), mid_y + s * 0.12),
                ],
                Stroke::new(lw * 2.0, w),
            );
            true
        }
        "files" => {
            let inner = rect.shrink(s * 0.22);
            painter.rect_stroke(
                inner,
                CornerRadius::same(2),
                Stroke::new(lw * 1.5, w),
                StrokeKind::Outside,
            );
            let tab = Rect::from_min_size(
                inner.left_top() - Vec2::new(0.0, s * 0.06),
                Vec2::new(inner.width() * 0.4, s * 0.06),
            );
            painter.rect_filled(
                tab,
                CornerRadius {
                    nw: 2,
                    ne: 2,
                    sw: 0,
                    se: 0,
                },
                w,
            );
            true
        }
        "calculator" | "calc" => {
            let inner = rect.shrink(s * 0.2);
            painter.rect_stroke(
                inner,
                CornerRadius::same(3),
                Stroke::new(lw * 1.5, w),
                StrokeKind::Outside,
            );
            // Grid dots
            let dx = inner.width() * 0.25;
            let dy = inner.height() * 0.25;
            for r in 1..=3 {
                for col in 1..=3 {
                    painter.circle_filled(
                        Pos2::new(inner.left() + col as f32 * dx, inner.top() + r as f32 * dy),
                        lw * 1.2,
                        w,
                    );
                }
            }
            true
        }
        "notes" | "notepad" => {
            let inner = rect.shrink(s * 0.2);
            painter.rect_stroke(
                inner,
                CornerRadius::same(2),
                Stroke::new(lw * 1.5, w),
                StrokeKind::Outside,
            );
            for i in 0..3 {
                let y = inner.top() + inner.height() * (0.25 + i as f32 * 0.22);
                painter.line_segment(
                    [
                        Pos2::new(inner.left() + s * 0.06, y),
                        Pos2::new(inner.right() - s * 0.06, y),
                    ],
                    Stroke::new(lw, w),
                );
            }
            true
        }
        "music" | "spotify" => {
            paint_media_icon(painter, rect);
            true
        }
        "photos" => {
            let inner = rect.shrink(s * 0.2);
            painter.rect_stroke(
                inner,
                CornerRadius::same(2),
                Stroke::new(lw * 1.5, w),
                StrokeKind::Outside,
            );
            // Mountain shape
            let base_y = inner.bottom() - s * 0.06;
            let peak1 = Pos2::new(
                inner.left() + inner.width() * 0.35,
                inner.top() + inner.height() * 0.4,
            );
            let peak2 = Pos2::new(
                inner.left() + inner.width() * 0.65,
                inner.top() + inner.height() * 0.55,
            );
            painter.line_segment(
                [Pos2::new(inner.left() + s * 0.06, base_y), peak1],
                Stroke::new(lw * 1.5, w),
            );
            painter.line_segment([peak1, peak2], Stroke::new(lw * 1.5, w));
            painter.line_segment(
                [peak2, Pos2::new(inner.right() - s * 0.06, base_y)],
                Stroke::new(lw * 1.5, w),
            );
            // Sun
            painter.circle_filled(
                Pos2::new(inner.right() - s * 0.15, inner.top() + s * 0.12),
                s * 0.06,
                w,
            );
            true
        }
        "calendar" => {
            let inner = rect.shrink(s * 0.2);
            painter.rect_stroke(
                inner,
                CornerRadius::same(2),
                Stroke::new(lw * 1.5, w),
                StrokeKind::Outside,
            );
            // Header bar
            let header = Rect::from_min_size(
                inner.left_top(),
                Vec2::new(inner.width(), inner.height() * 0.25),
            );
            painter.rect_filled(
                header,
                CornerRadius {
                    nw: 2,
                    ne: 2,
                    sw: 0,
                    se: 0,
                },
                Color32::from_white_alpha(60),
            );
            // Day number
            painter.text(
                Pos2::new(c.x, inner.top() + inner.height() * 0.6),
                Align2::CENTER_CENTER,
                chrono::Local::now().format("%d").to_string(),
                FontId::proportional(s * 0.25),
                w,
            );
            true
        }
        "browser" | "safari" | "google chrome" | "firefox" | "microsoft edge" | "brave" => {
            paint_globe_icon(painter, rect);
            true
        }
        "settings" | "system preferences" => {
            paint_gear_icon(painter, rect);
            true
        }
        "activity monitor" | "task manager" => {
            let inner = rect.shrink(s * 0.2);
            // CPU graph lines
            let steps = 8;
            let heights = [0.5, 0.7, 0.3, 0.8, 0.4, 0.6, 0.9, 0.5];
            for i in 0..steps - 1 {
                let x1 = inner.left() + i as f32 / (steps - 1) as f32 * inner.width();
                let x2 = inner.left() + (i + 1) as f32 / (steps - 1) as f32 * inner.width();
                let y1 = inner.bottom() - heights[i] * inner.height();
                let y2 = inner.bottom() - heights[i + 1] * inner.height();
                painter.line_segment(
                    [Pos2::new(x1, y1), Pos2::new(x2, y2)],
                    Stroke::new(lw * 2.0, w),
                );
            }
            true
        }
        "messages" => {
            paint_chat_icon(painter, rect);
            true
        }
        "system overview" => {
            // Dashboard gauge
            let r = s * 0.3;
            painter.circle_stroke(c, r, Stroke::new(lw * 2.0, w));
            // Needle
            let angle: f32 = -std::f32::consts::FRAC_PI_4;
            painter.line_segment(
                [
                    c,
                    Pos2::new(c.x + angle.cos() * r * 0.7, c.y + angle.sin() * r * 0.7),
                ],
                Stroke::new(lw * 2.0, w),
            );
            true
        }
        _ => false,
    }
}

// ── Category generic icons ──────────────────────────────────────────────────

fn paint_dev_icon(painter: &Painter, rect: Rect) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let w = Color32::WHITE;
    let lw = (s * 0.04).max(1.5);
    // </>
    let h = s * 0.18;
    // <
    painter.line_segment(
        [
            Pos2::new(c.x - s * 0.15, c.y - h),
            Pos2::new(c.x - s * 0.28, c.y),
        ],
        Stroke::new(lw * 2.0, w),
    );
    painter.line_segment(
        [
            Pos2::new(c.x - s * 0.28, c.y),
            Pos2::new(c.x - s * 0.15, c.y + h),
        ],
        Stroke::new(lw * 2.0, w),
    );
    // /
    painter.line_segment(
        [
            Pos2::new(c.x + s * 0.05, c.y - h),
            Pos2::new(c.x - s * 0.05, c.y + h),
        ],
        Stroke::new(lw * 1.5, w),
    );
    // >
    painter.line_segment(
        [
            Pos2::new(c.x + s * 0.15, c.y - h),
            Pos2::new(c.x + s * 0.28, c.y),
        ],
        Stroke::new(lw * 2.0, w),
    );
    painter.line_segment(
        [
            Pos2::new(c.x + s * 0.28, c.y),
            Pos2::new(c.x + s * 0.15, c.y + h),
        ],
        Stroke::new(lw * 2.0, w),
    );
}

fn paint_globe_icon(painter: &Painter, rect: Rect) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let w = Color32::WHITE;
    let lw = (s * 0.04).max(1.0);
    let r = s * 0.3;
    painter.circle_stroke(c, r, Stroke::new(lw * 1.5, w));
    // Horizontal line
    painter.line_segment(
        [Pos2::new(c.x - r, c.y), Pos2::new(c.x + r, c.y)],
        Stroke::new(lw, w),
    );
    // Vertical ellipse (meridian)
    let steps = 12;
    for i in 0..steps {
        let a1 = i as f32 / steps as f32 * std::f32::consts::TAU;
        let a2 = (i + 1) as f32 / steps as f32 * std::f32::consts::TAU;
        painter.line_segment(
            [
                Pos2::new(c.x + a1.cos() * r * 0.4, c.y + a1.sin() * r),
                Pos2::new(c.x + a2.cos() * r * 0.4, c.y + a2.sin() * r),
            ],
            Stroke::new(lw, w),
        );
    }
}

fn paint_game_icon(painter: &Painter, rect: Rect) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let w = Color32::WHITE;
    let lw = (s * 0.04).max(1.5);
    // Gamepad body (rounded rect)
    let body = Rect::from_center_size(c, Vec2::new(s * 0.55, s * 0.3));
    painter.rect_stroke(
        body,
        CornerRadius::same(6),
        Stroke::new(lw * 1.5, w),
        StrokeKind::Outside,
    );
    // D-pad (left)
    let dl = Pos2::new(c.x - s * 0.14, c.y);
    painter.line_segment(
        [
            Pos2::new(dl.x - s * 0.06, dl.y),
            Pos2::new(dl.x + s * 0.06, dl.y),
        ],
        Stroke::new(lw * 1.5, w),
    );
    painter.line_segment(
        [
            Pos2::new(dl.x, dl.y - s * 0.06),
            Pos2::new(dl.x, dl.y + s * 0.06),
        ],
        Stroke::new(lw * 1.5, w),
    );
    // Buttons (right)
    painter.circle_filled(Pos2::new(c.x + s * 0.12, c.y - s * 0.03), s * 0.025, w);
    painter.circle_filled(Pos2::new(c.x + s * 0.18, c.y), s * 0.025, w);
}

fn paint_media_icon(painter: &Painter, rect: Rect) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let w = Color32::WHITE;
    // Play triangle
    let size = s * 0.25;
    let points = vec![
        Pos2::new(c.x - size * 0.4, c.y - size),
        Pos2::new(c.x + size * 0.8, c.y),
        Pos2::new(c.x - size * 0.4, c.y + size),
    ];
    painter.add(eframe::egui::Shape::convex_polygon(points, w, Stroke::NONE));
}

fn paint_chat_icon(painter: &Painter, rect: Rect) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let w = Color32::WHITE;
    let lw = (s * 0.04).max(1.0);
    // Speech bubble
    let body = Rect::from_center_size(Pos2::new(c.x, c.y - s * 0.04), Vec2::new(s * 0.5, s * 0.35));
    painter.rect_stroke(
        body,
        CornerRadius::same(6),
        Stroke::new(lw * 1.5, w),
        StrokeKind::Outside,
    );
    // Tail
    let tail_x = body.left() + body.width() * 0.25;
    painter.line_segment(
        [
            Pos2::new(tail_x, body.bottom()),
            Pos2::new(tail_x - s * 0.06, body.bottom() + s * 0.08),
        ],
        Stroke::new(lw * 1.5, w),
    );
    painter.line_segment(
        [
            Pos2::new(tail_x - s * 0.06, body.bottom() + s * 0.08),
            Pos2::new(tail_x + s * 0.06, body.bottom()),
        ],
        Stroke::new(lw * 1.5, w),
    );
}

fn paint_wrench_icon(painter: &Painter, rect: Rect) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let w = Color32::WHITE;
    let lw = (s * 0.04).max(1.5);
    // Wrench handle (diagonal line)
    painter.line_segment(
        [
            Pos2::new(c.x - s * 0.2, c.y + s * 0.2),
            Pos2::new(c.x + s * 0.1, c.y - s * 0.1),
        ],
        Stroke::new(lw * 2.5, w),
    );
    // Wrench head (circle at top)
    painter.circle_stroke(
        Pos2::new(c.x + s * 0.15, c.y - s * 0.15),
        s * 0.1,
        Stroke::new(lw * 2.0, w),
    );
}

fn paint_doc_icon(painter: &Painter, rect: Rect) {
    let s = rect.width().min(rect.height());
    let w = Color32::WHITE;
    let lw = (s * 0.04).max(1.0);
    let inner = rect.shrink(s * 0.22);
    // Document outline
    painter.rect_stroke(
        inner,
        CornerRadius::same(2),
        Stroke::new(lw * 1.5, w),
        StrokeKind::Outside,
    );
    // Lines of text
    for i in 0..4 {
        let y = inner.top() + inner.height() * (0.2 + i as f32 * 0.18);
        let end_x = if i == 3 {
            inner.left() + inner.width() * 0.5
        } else {
            inner.right() - s * 0.06
        };
        painter.line_segment(
            [Pos2::new(inner.left() + s * 0.06, y), Pos2::new(end_x, y)],
            Stroke::new(lw, w),
        );
    }
}

fn paint_palette_icon(painter: &Painter, rect: Rect) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let w = Color32::WHITE;
    let lw = (s * 0.04).max(1.0);
    // Palette outline (circle)
    painter.circle_stroke(c, s * 0.3, Stroke::new(lw * 1.5, w));
    // Color dots
    let colors = [
        Color32::from_rgb(255, 59, 48),
        Color32::from_rgb(52, 199, 89),
        Color32::from_rgb(0, 122, 255),
        Color32::from_rgb(255, 214, 10),
    ];
    for (i, color) in colors.iter().enumerate() {
        let angle = i as f32 * std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4;
        let dx = angle.cos() * s * 0.17;
        let dy = angle.sin() * s * 0.17;
        painter.circle_filled(Pos2::new(c.x + dx, c.y + dy), s * 0.05, *color);
    }
}

fn paint_gear_icon(painter: &Painter, rect: Rect) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let w = Color32::WHITE;
    let lw = (s * 0.04).max(1.5);
    let r = s * 0.22;
    painter.circle_stroke(c, r, Stroke::new(lw * 1.5, w));
    // Gear teeth
    let teeth = 8;
    for i in 0..teeth {
        let angle = i as f32 / teeth as f32 * std::f32::consts::TAU;
        painter.line_segment(
            [
                Pos2::new(c.x + angle.cos() * r, c.y + angle.sin() * r),
                Pos2::new(
                    c.x + angle.cos() * (r + s * 0.08),
                    c.y + angle.sin() * (r + s * 0.08),
                ),
            ],
            Stroke::new(lw * 2.0, w),
        );
    }
    painter.circle_filled(c, s * 0.06, w);
}

/// Map a category name to an icon type string (for testing).
pub fn icon_type_for_category(category: &str) -> &'static str {
    match category {
        "Development" => "code_brackets",
        "Internet" => "globe",
        "Games" => "gamepad",
        "Media" => "play",
        "Communication" => "chat_bubble",
        "Utilities" => "wrench",
        "Productivity" => "document",
        "Graphics" => "palette",
        "System" => "gear",
        _ => "text_fallback",
    }
}

/// Check if a builtin name has a dedicated icon.
pub fn has_builtin_icon(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "terminal"
            | "files"
            | "calculator"
            | "calc"
            | "notes"
            | "notepad"
            | "music"
            | "spotify"
            | "photos"
            | "calendar"
            | "browser"
            | "safari"
            | "google chrome"
            | "firefox"
            | "microsoft edge"
            | "brave"
            | "settings"
            | "system preferences"
            | "activity monitor"
            | "task manager"
            | "messages"
            | "system overview"
    )
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_categories_have_icon_type() {
        let cats = [
            "Development",
            "Internet",
            "Games",
            "Media",
            "Communication",
            "Utilities",
            "Productivity",
            "Graphics",
            "System",
            "Unknown",
        ];
        for cat in cats {
            let t = icon_type_for_category(cat);
            assert!(!t.is_empty(), "Category '{}' should have icon type", cat);
        }
    }

    #[test]
    fn unknown_category_is_text_fallback() {
        assert_eq!(icon_type_for_category("SomethingRandom"), "text_fallback");
    }

    #[test]
    fn builtin_icons_recognized() {
        assert!(has_builtin_icon("Terminal"));
        assert!(has_builtin_icon("Files"));
        assert!(has_builtin_icon("Calculator"));
        assert!(has_builtin_icon("Notes"));
        assert!(has_builtin_icon("Music"));
        assert!(has_builtin_icon("Photos"));
        assert!(has_builtin_icon("Calendar"));
        assert!(has_builtin_icon("Browser"));
        assert!(has_builtin_icon("Settings"));
        assert!(has_builtin_icon("Messages"));
    }

    #[test]
    fn builtin_icons_case_insensitive() {
        assert!(has_builtin_icon("terminal"));
        assert!(has_builtin_icon("TERMINAL"));
        assert!(has_builtin_icon("Google Chrome"));
    }

    #[test]
    fn unknown_app_not_builtin() {
        assert!(!has_builtin_icon("RandomApp123"));
        assert!(!has_builtin_icon("MyCustomTool"));
    }

    #[test]
    fn each_category_maps_differently() {
        let cats = [
            "Development",
            "Internet",
            "Games",
            "Media",
            "Communication",
            "Utilities",
            "Productivity",
            "Graphics",
            "System",
        ];
        let types: Vec<&str> = cats.iter().map(|c| icon_type_for_category(c)).collect();
        // All should be unique
        let mut unique = types.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            types.len(),
            unique.len(),
            "Each category should map to a unique icon type"
        );
    }
}
