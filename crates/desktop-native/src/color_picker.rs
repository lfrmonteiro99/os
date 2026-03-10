use eframe::egui::{self, Align, Color32, CornerRadius, RichText, Stroke, Vec2};

use crate::clipboard::AppClipboard;

const DEFAULT_SWATCHES: &[(u8, u8, u8)] = &[
    (255, 59, 48),
    (255, 149, 0),
    (255, 204, 0),
    (52, 199, 89),
    (0, 199, 190),
    (0, 122, 255),
    (88, 86, 214),
    (191, 90, 242),
    (172, 142, 104),
    (142, 142, 147),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SliderMode {
    Rgb,
    Hsb,
    Cmyk,
}

impl SliderMode {
    fn label(self) -> &'static str {
        match self {
            Self::Rgb => "RGB",
            Self::Hsb => "HSB",
            Self::Cmyk => "CMYK",
        }
    }
}

pub struct ColorPickerApp {
    color: Color32,
    hex_input: String,
    favorites: Vec<Color32>,
    slider_mode: SliderMode,
    eyedropper_active: bool,
    keep_on_top: bool,
    status: String,
    last_sampled: Option<Color32>,
}

impl ColorPickerApp {
    pub fn new(saved_colors: &str, initial: Color32) -> Self {
        Self {
            color: initial,
            hex_input: color_to_hex(initial),
            favorites: parse_saved_colors(saved_colors),
            slider_mode: SliderMode::Rgb,
            eyedropper_active: false,
            keep_on_top: false,
            status: "Pick a color and copy any format.".to_string(),
            last_sampled: None,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, clipboard: &AppClipboard) -> bool {
        let white = Color32::from_gray(235);
        let gray = Color32::from_gray(150);
        let panel = Color32::from_rgba_unmultiplied(255, 255, 255, 10);
        let mut request_save = false;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Color Picker").size(16.0).strong().color(white));
                        ui.label(
                            RichText::new(
                                "Floating utility for picking, copying, and saving AuroraOS colors.",
                            )
                            .size(11.0)
                            .color(gray),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.checkbox(&mut self.keep_on_top, "Always on top");
                    });
                });

                ui.add_space(8.0);
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Preview").size(12.0).strong().color(white));
                                ui.add_space(8.0);
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 110.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, CornerRadius::same(18), self.color);
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    color_to_hex(self.color),
                                    egui::FontId::proportional(18.0),
                                    contrast_color(self.color),
                                );
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Wheel").size(11.0).color(gray));
                                    ui.color_edit_button_srgba(&mut self.color);
                                    if ui
                                        .button(if self.eyedropper_active {
                                            "Cancel Eyedropper"
                                        } else {
                                            "Eyedropper"
                                        })
                                        .clicked()
                                    {
                                        self.eyedropper_active = !self.eyedropper_active;
                                        self.status = if self.eyedropper_active {
                                            "Eyedropper active. Click anywhere on the desktop preview.".to_string()
                                        } else {
                                            "Eyedropper cancelled.".to_string()
                                        };
                                    }
                                });
                                if let Some(sample) = self.last_sampled {
                                    ui.add_space(8.0);
                                    render_magnifier(ui, sample);
                                }
                            });

                        ui.add_space(8.0);
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Formats").size(12.0).strong().color(white));
                                ui.add_space(6.0);
                                let rgb = color_to_rgb_string(self.color);
                                let hsb = color_to_hsb_string(self.color);
                                let cmyk = color_to_cmyk_string(self.color);
                                let hex = color_to_hex(self.color);
                                for (label, value) in [("Hex", hex), ("RGB", rgb), ("HSB", hsb), ("CMYK", cmyk)] {
                                    ui.horizontal(|ui| {
                                        if ui.button(format!("Copy {label}")).clicked() {
                                            clipboard.copy(&value);
                                            self.status = format!("Copied {label}: {value}");
                                        }
                                        ui.label(RichText::new(value).size(11.0).color(Color32::WHITE));
                                    });
                                }
                            });
                    });

                    columns[1].vertical(|ui| {
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Controls").size(12.0).strong().color(white));
                                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                        for mode in [SliderMode::Rgb, SliderMode::Hsb, SliderMode::Cmyk] {
                                            let selected = self.slider_mode == mode;
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(mode.label())
                                                            .size(10.0)
                                                            .color(Color32::WHITE),
                                                    )
                                                    .fill(if selected {
                                                        Color32::from_rgb(0, 122, 255)
                                                    } else {
                                                        Color32::from_gray(55)
                                                    })
                                                    .corner_radius(CornerRadius::same(8)),
                                                )
                                                .clicked()
                                            {
                                                self.slider_mode = mode;
                                            }
                                        }
                                    });
                                });
                                ui.add_space(6.0);
                                self.render_hex_editor(ui, gray);
                                ui.add_space(6.0);
                                self.render_slider_controls(ui);
                            });

                        ui.add_space(8.0);
                        egui::Frame::default()
                            .fill(panel)
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Palette").size(12.0).strong().color(white));
                                ui.add_space(6.0);
                                render_palette_row(ui, DEFAULT_SWATCHES, &mut self.color);
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Favorites").size(12.0).strong().color(white));
                                    if ui.button("Save Current").clicked() && !self.favorites.contains(&self.color) {
                                        self.favorites.push(self.color);
                                        request_save = true;
                                        self.status = format!("Saved {} to favorites.", color_to_hex(self.color));
                                    }
                                });
                                if self.favorites.is_empty() {
                                    ui.label(
                                        RichText::new("No saved colors yet.")
                                            .size(10.0)
                                            .color(gray),
                                    );
                                } else {
                                    let favorites = self.favorites.clone();
                                    ui.horizontal_wrapped(|ui| {
                                        for favorite in favorites {
                                            let swatch = color_swatch_button(ui, favorite, favorite == self.color);
                                            if swatch.clicked() {
                                                self.color = favorite;
                                            }
                                            if swatch.secondary_clicked() {
                                                self.favorites.retain(|color| *color != favorite);
                                                request_save = true;
                                                self.status = "Removed color from favorites.".to_string();
                                            }
                                        }
                                    });
                                }
                            });
                    });
                });

                ui.add_space(8.0);
                ui.label(RichText::new(&self.status).size(10.0).color(gray));
            });

        self.hex_input = color_to_hex(self.color);
        request_save
    }

    pub fn apply_sample(&mut self, color: Color32) {
        self.color = color;
        self.last_sampled = Some(color);
        self.eyedropper_active = false;
        self.status = format!("Sampled {} from desktop.", color_to_hex(color));
        self.hex_input = color_to_hex(color);
    }

    pub fn selected_color(&self) -> Color32 {
        self.color
    }

    pub fn eyedropper_active(&self) -> bool {
        self.eyedropper_active
    }

    pub fn keep_on_top(&self) -> bool {
        self.keep_on_top
    }

    pub fn serialized_favorites(&self) -> String {
        self.favorites
            .iter()
            .map(|color| color_to_hex(*color))
            .collect::<Vec<_>>()
            .join("|")
    }

    fn render_hex_editor(&mut self, ui: &mut egui::Ui, gray: Color32) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Hex").size(11.0).color(gray));
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.hex_input)
                    .hint_text("#RRGGBB")
                    .desired_width(120.0),
            );
            if response.changed() {
                if let Some(color) = parse_hex_color(&self.hex_input) {
                    self.color = color;
                    self.status = format!("Parsed {}.", color_to_hex(color));
                }
            }
        });
    }

    fn render_slider_controls(&mut self, ui: &mut egui::Ui) {
        match self.slider_mode {
            SliderMode::Rgb => {
                let (mut r, mut g, mut b) = (self.color.r(), self.color.g(), self.color.b());
                slider_u8(ui, "R", &mut r);
                slider_u8(ui, "G", &mut g);
                slider_u8(ui, "B", &mut b);
                self.color = Color32::from_rgb(r, g, b);
            }
            SliderMode::Hsb => {
                let (mut h, mut s, mut v) = rgb_to_hsv(self.color);
                slider_f32(ui, "H", &mut h, 0.0, 360.0);
                slider_f32(ui, "S", &mut s, 0.0, 100.0);
                slider_f32(ui, "B", &mut v, 0.0, 100.0);
                self.color = hsv_to_color(h, s, v);
            }
            SliderMode::Cmyk => {
                let (mut c, mut m, mut y, mut k) = rgb_to_cmyk(self.color);
                slider_f32(ui, "C", &mut c, 0.0, 100.0);
                slider_f32(ui, "M", &mut m, 0.0, 100.0);
                slider_f32(ui, "Y", &mut y, 0.0, 100.0);
                slider_f32(ui, "K", &mut k, 0.0, 100.0);
                self.color = cmyk_to_color(c, m, y, k);
            }
        }
    }
}

fn slider_u8(ui: &mut egui::Ui, label: &str, value: &mut u8) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut current = *value as i32;
        ui.add(egui::Slider::new(&mut current, 0..=255).show_value(true));
        *value = current.clamp(0, 255) as u8;
    });
}

fn slider_f32(ui: &mut egui::Ui, label: &str, value: &mut f32, min: f32, max: f32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(value, min..=max).show_value(true));
    });
}

fn render_palette_row(ui: &mut egui::Ui, colors: &[(u8, u8, u8)], selected: &mut Color32) {
    ui.horizontal_wrapped(|ui| {
        for &(r, g, b) in colors {
            let color = Color32::from_rgb(r, g, b);
            if color_swatch_button(ui, color, *selected == color).clicked() {
                *selected = color;
            }
        }
    });
}

fn color_swatch_button(ui: &mut egui::Ui, color: Color32, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::click());
    let stroke = if selected {
        Stroke::new(2.0, Color32::WHITE)
    } else {
        Stroke::new(1.0, Color32::from_white_alpha(40))
    };
    ui.painter()
        .rect(rect, CornerRadius::same(8), color, stroke, egui::StrokeKind::Outside);
    response
}

fn render_magnifier(ui: &mut egui::Ui, color: Color32) {
    ui.label(RichText::new("Eyedropper Sample").size(10.0).strong());
    egui::Grid::new("color-picker-magnifier")
        .num_columns(5)
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            for row in 0..5 {
                for col in 0..5 {
                    let factor = 0.75 + ((row + col) as f32 * 0.025);
                    let swatch = tint_color(color, factor);
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::splat(if row == 2 && col == 2 { 18.0 } else { 14.0 }), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(4), swatch);
                }
                ui.end_row();
            }
        });
}

fn parse_saved_colors(saved: &str) -> Vec<Color32> {
    saved
        .split('|')
        .filter_map(parse_hex_color)
        .collect()
}

pub fn sample_color_from_position(position: egui::Pos2, rect: egui::Rect) -> Color32 {
    let x = ((position.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
    let y = ((position.y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
    let hue = x * 360.0;
    let brightness = (1.0 - y * 0.55).clamp(0.2, 1.0) * 100.0;
    let saturation = (0.35 + y * 0.65).clamp(0.0, 1.0) * 100.0;
    hsv_to_color(hue, saturation, brightness)
}

fn color_to_hex(color: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
}

fn parse_hex_color(value: &str) -> Option<Color32> {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
    let g = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
    let b = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
    Some(Color32::from_rgb(r, g, b))
}

fn color_to_rgb_string(color: Color32) -> String {
    format!("rgb({}, {}, {})", color.r(), color.g(), color.b())
}

fn color_to_hsb_string(color: Color32) -> String {
    let (h, s, v) = rgb_to_hsv(color);
    format!("hsb({:.0}, {:.0}%, {:.0}%)", h, s, v)
}

fn color_to_cmyk_string(color: Color32) -> String {
    let (c, m, y, k) = rgb_to_cmyk(color);
    format!("cmyk({:.0}%, {:.0}%, {:.0}%, {:.0}%)", c, m, y, k)
}

fn contrast_color(color: Color32) -> Color32 {
    let luma = 0.299 * color.r() as f32 + 0.587 * color.g() as f32 + 0.114 * color.b() as f32;
    if luma > 170.0 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

fn tint_color(color: Color32, factor: f32) -> Color32 {
    Color32::from_rgb(
        (color.r() as f32 * factor).clamp(0.0, 255.0) as u8,
        (color.g() as f32 * factor).clamp(0.0, 255.0) as u8,
        (color.b() as f32 * factor).clamp(0.0, 255.0) as u8,
    )
}

fn rgb_to_hsv(color: Color32) -> (f32, f32, f32) {
    let r = color.r() as f32 / 255.0;
    let g = color.g() as f32 / 255.0;
    let b = color.b() as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let saturation = if max == 0.0 { 0.0 } else { (delta / max) * 100.0 };
    let value = max * 100.0;
    (hue, saturation, value)
}

fn hsv_to_color(h: f32, s: f32, v: f32) -> Color32 {
    let h = h.rem_euclid(360.0);
    let s = (s / 100.0).clamp(0.0, 1.0);
    let v = (v / 100.0).clamp(0.0, 1.0);
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0).rem_euclid(2.0)) - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color32::from_rgb(
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

fn rgb_to_cmyk(color: Color32) -> (f32, f32, f32, f32) {
    let r = color.r() as f32 / 255.0;
    let g = color.g() as f32 / 255.0;
    let b = color.b() as f32 / 255.0;
    let k = 1.0 - r.max(g).max(b);
    if k >= 0.999 {
        return (0.0, 0.0, 0.0, 100.0);
    }
    let c = ((1.0 - r - k) / (1.0 - k) * 100.0).clamp(0.0, 100.0);
    let m = ((1.0 - g - k) / (1.0 - k) * 100.0).clamp(0.0, 100.0);
    let y = ((1.0 - b - k) / (1.0 - k) * 100.0).clamp(0.0, 100.0);
    (c, m, y, k * 100.0)
}

fn cmyk_to_color(c: f32, m: f32, y: f32, k: f32) -> Color32 {
    let c = (c / 100.0).clamp(0.0, 1.0);
    let m = (m / 100.0).clamp(0.0, 1.0);
    let y = (y / 100.0).clamp(0.0, 1.0);
    let k = (k / 100.0).clamp(0.0, 1.0);
    Color32::from_rgb(
        ((1.0 - c) * (1.0 - k) * 255.0).round() as u8,
        ((1.0 - m) * (1.0 - k) * 255.0).round() as u8,
        ((1.0 - y) * (1.0 - k) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip_works() {
        let color = Color32::from_rgb(255, 122, 30);
        assert_eq!(parse_hex_color(&color_to_hex(color)), Some(color));
    }

    #[test]
    fn hsv_roundtrip_is_close() {
        let color = Color32::from_rgb(64, 128, 220);
        let (h, s, v) = rgb_to_hsv(color);
        let roundtrip = hsv_to_color(h, s, v);
        assert!((roundtrip.r() as i32 - color.r() as i32).abs() <= 1);
        assert!((roundtrip.g() as i32 - color.g() as i32).abs() <= 1);
        assert!((roundtrip.b() as i32 - color.b() as i32).abs() <= 1);
    }

    #[test]
    fn cmyk_roundtrip_is_close() {
        let color = Color32::from_rgb(20, 180, 90);
        let (c, m, y, k) = rgb_to_cmyk(color);
        let roundtrip = cmyk_to_color(c, m, y, k);
        assert!((roundtrip.r() as i32 - color.r() as i32).abs() <= 1);
        assert!((roundtrip.g() as i32 - color.g() as i32).abs() <= 1);
        assert!((roundtrip.b() as i32 - color.b() as i32).abs() <= 1);
    }

    #[test]
    fn saved_colors_parse_pipe_delimited_hex() {
        let parsed = parse_saved_colors("#FF0000|#00FF00|#0000FF");
        assert_eq!(parsed.len(), 3);
    }

    #[test]
    fn sample_color_changes_with_position() {
        let rect = egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), Vec2::new(100.0, 100.0));
        let a = sample_color_from_position(egui::Pos2::new(10.0, 10.0), rect);
        let b = sample_color_from_position(egui::Pos2::new(90.0, 90.0), rect);
        assert_ne!(a, b);
    }
}
