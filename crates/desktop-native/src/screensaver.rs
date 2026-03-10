use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Local;
use eframe::egui::{self, Align2, Color32, FontId, Id, Order, Pos2, Rect, Stroke};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenSaverKind {
    FloatingClock,
    Aurora,
    Matrix,
    PhotoSlideshow,
    Blank,
}

impl ScreenSaverKind {
    pub fn next(self) -> Self {
        match self {
            Self::FloatingClock => Self::Aurora,
            Self::Aurora => Self::Matrix,
            Self::Matrix => Self::PhotoSlideshow,
            Self::PhotoSlideshow => Self::Blank,
            Self::Blank => Self::FloatingClock,
        }
    }
}

pub fn photo_slideshow_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| is_photo_path(path))
        .cloned()
        .collect()
}

pub fn is_photo_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"
            )
        })
        .unwrap_or(false)
}

pub fn render_screensaver_overlay(
    ctx: &egui::Context,
    kind: ScreenSaverKind,
    elapsed: Duration,
    photos: &[PathBuf],
) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        Order::Foreground,
        Id::new("screensaver"),
    ));
    painter.rect_filled(screen, 0.0, Color32::BLACK);

    match kind {
        ScreenSaverKind::FloatingClock => render_floating_clock(&painter, screen, elapsed),
        ScreenSaverKind::Aurora => render_aurora(&painter, screen, elapsed),
        ScreenSaverKind::Matrix => render_matrix(&painter, screen, elapsed),
        ScreenSaverKind::PhotoSlideshow => {
            render_photo_slideshow(&painter, screen, elapsed, photos)
        }
        ScreenSaverKind::Blank => {}
    }
}

fn render_floating_clock(painter: &egui::Painter, screen: Rect, elapsed: Duration) {
    let secs = elapsed.as_secs_f32();
    let x = screen.left() + (secs.sin() * 0.5 + 0.5) * (screen.width() - 280.0) + 140.0;
    let y = screen.top() + (secs.cos() * 0.5 + 0.5) * (screen.height() - 120.0) + 60.0;
    painter.text(
        Pos2::new(x, y),
        Align2::CENTER_CENTER,
        Local::now().format("%H:%M").to_string(),
        FontId::proportional(64.0),
        Color32::from_gray(235),
    );
    painter.text(
        Pos2::new(x, y + 50.0),
        Align2::CENTER_CENTER,
        Local::now().format("%A, %B %d").to_string(),
        FontId::proportional(18.0),
        Color32::from_gray(150),
    );
}

fn render_aurora(painter: &egui::Painter, screen: Rect, elapsed: Duration) {
    let t = elapsed.as_secs_f32();
    for band in 0..7 {
        let progress = band as f32 / 6.0;
        let y = screen.top() + progress * screen.height();
        let phase = t * 0.4 + band as f32 * 0.7;
        let left = Pos2::new(screen.left(), y + phase.sin() * 30.0);
        let right = Pos2::new(screen.right(), y + phase.cos() * 30.0);
        let color = Color32::from_rgba_unmultiplied(
            (20.0 + progress * 40.0) as u8,
            (80.0 + progress * 120.0) as u8,
            (90.0 + progress * 60.0) as u8,
            120,
        );
        painter.line_segment([left, right], Stroke::new(48.0, color));
    }
}

fn render_matrix(painter: &egui::Painter, screen: Rect, elapsed: Duration) {
    let chars = ["0", "1", "A", "U", "R", "O", "S"];
    let step_x = 26.0;
    let cols = (screen.width() / step_x).floor() as usize;
    let t = elapsed.as_secs_f32();
    for col in 0..cols {
        let x = screen.left() + col as f32 * step_x + 10.0;
        let offset =
            ((t * 80.0) as i32 + col as i32 * 13).rem_euclid(screen.height() as i32 + 120) as f32;
        for row in 0..6 {
            let y = screen.top() + (offset - row as f32 * 22.0);
            if y < screen.top() - 20.0 || y > screen.bottom() + 20.0 {
                continue;
            }
            let alpha = 220_u8.saturating_sub((row * 35) as u8);
            painter.text(
                Pos2::new(x, y),
                Align2::CENTER_CENTER,
                chars[(col + row) % chars.len()],
                FontId::monospace(18.0),
                Color32::from_rgba_unmultiplied(80, 255, 120, alpha),
            );
        }
    }
}

fn render_photo_slideshow(
    painter: &egui::Painter,
    screen: Rect,
    elapsed: Duration,
    photos: &[PathBuf],
) {
    if photos.is_empty() {
        render_floating_clock(painter, screen, elapsed);
        return;
    }

    let cycle = ((elapsed.as_secs_f32() / 6.0).floor() as usize) % photos.len();
    let label = photos[cycle]
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Photo");
    let t = elapsed.as_secs_f32();
    let pan = ((t * 0.18).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
    let zoom = 1.04 + ((t * 0.12).cos() * 0.03);
    let hash = label.bytes().fold(0u32, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as u32)
    });
    let top = Color32::from_rgb(
        40 + (hash % 90) as u8,
        70 + ((hash / 7) % 110) as u8,
        100 + ((hash / 17) % 120) as u8,
    );
    let bottom = Color32::from_rgb(
        20 + ((hash / 5) % 80) as u8,
        35 + ((hash / 11) % 90) as u8,
        60 + ((hash / 23) % 100) as u8,
    );

    let inset_x = 40.0 + pan * 24.0;
    let inset_y = 36.0 + (1.0 - pan) * 20.0;
    let image_rect = Rect::from_min_max(
        Pos2::new(screen.left() + inset_x, screen.top() + inset_y),
        Pos2::new(
            screen.right() - inset_x / zoom,
            screen.bottom() - inset_y / zoom,
        ),
    );

    painter.rect_filled(screen, 0.0, Color32::BLACK);
    painter.rect_filled(image_rect, 18.0, top);
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(image_rect.left(), image_rect.center().y),
            image_rect.right_bottom(),
        ),
        18.0,
        bottom,
    );
    painter.rect_stroke(
        image_rect,
        18.0,
        Stroke::new(1.0, Color32::from_white_alpha(36)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        Pos2::new(image_rect.left() + 22.0, image_rect.bottom() - 30.0),
        Align2::LEFT_BOTTOM,
        label,
        FontId::proportional(22.0),
        Color32::from_gray(245),
    );
    painter.text(
        Pos2::new(image_rect.left() + 22.0, image_rect.bottom() - 10.0),
        Align2::LEFT_BOTTOM,
        format!("Photo {} of {}", cycle + 1, photos.len()),
        FontId::proportional(12.0),
        Color32::from_gray(190),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn screensaver_kind_cycles() {
        assert_eq!(
            ScreenSaverKind::FloatingClock.next(),
            ScreenSaverKind::Aurora
        );
        assert_eq!(ScreenSaverKind::Aurora.next(), ScreenSaverKind::Matrix);
        assert_eq!(
            ScreenSaverKind::Matrix.next(),
            ScreenSaverKind::PhotoSlideshow
        );
        assert_eq!(
            ScreenSaverKind::PhotoSlideshow.next(),
            ScreenSaverKind::Blank
        );
        assert_eq!(
            ScreenSaverKind::Blank.next(),
            ScreenSaverKind::FloatingClock
        );
    }

    #[test]
    fn photo_slideshow_paths_filter_supported_images() {
        let paths = vec![
            PathBuf::from("C:/Pictures/a.png"),
            PathBuf::from("C:/Pictures/b.jpg"),
            PathBuf::from("C:/Pictures/c.txt"),
            PathBuf::from("C:/Pictures/d.JPEG"),
        ];
        let filtered = photo_slideshow_paths(&paths);
        assert_eq!(
            filtered,
            vec![
                PathBuf::from("C:/Pictures/a.png"),
                PathBuf::from("C:/Pictures/b.jpg"),
                PathBuf::from("C:/Pictures/d.JPEG"),
            ]
        );
    }
}
