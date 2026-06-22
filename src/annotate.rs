use image::{DynamicImage, Rgb, RgbImage};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;

use crate::types::{ExtractResult, UiElement, UiElementKind};

const CONTAINER_COLOR: Rgb<u8> = Rgb([0, 120, 255]);
const TEXT_COLOR: Rgb<u8> = Rgb([0, 180, 70]);
const ICON_COLOR: Rgb<u8> = Rgb([255, 140, 0]);

/// Draw bounding boxes for a layout tree on top of the source screenshot.
pub fn render_layout_annotation(source: &DynamicImage, root: &UiElement) -> RgbImage {
    let result = ExtractResult {
        width: source.width() as i32,
        height: source.height() as i32,
        root: root.clone(),
    };
    render_annotation(source, &result)
}

/// Draw bounding boxes and labels on top of the source screenshot.
pub fn render_annotation(source: &DynamicImage, result: &ExtractResult) -> RgbImage {
    let mut canvas = source.to_rgb8();
    draw_element(&mut canvas, &result.root, true);
    canvas
}

fn draw_element(canvas: &mut RgbImage, element: &UiElement, is_root: bool) {
    if !is_root {
        let color = match element.kind {
            UiElementKind::Container => CONTAINER_COLOR,
            UiElementKind::Text { .. } => TEXT_COLOR,
            UiElementKind::Icon { .. } => ICON_COLOR,
            UiElementKind::Root => return,
        };
        draw_bounds(canvas, &element.bounds, color);

        match &element.kind {
            UiElementKind::Text { content, .. } => {
                draw_label(canvas, &element.bounds, content, color);
            }
            UiElementKind::Icon { name, .. } => {
                draw_label(canvas, &element.bounds, name, color);
            }
            _ => {}
        }
    }

    for child in &element.children {
        draw_element(canvas, child, false);
    }
}

fn draw_bounds(canvas: &mut RgbImage, bounds: &crate::types::Bounds, color: Rgb<u8>) {
    let rect = Rect::at(bounds.x, bounds.y).of_size(bounds.width as u32, bounds.height as u32);
    for offset in 0..2 {
        let inset = Rect::at(bounds.x + offset, bounds.y + offset).of_size(
            (bounds.width - offset * 2).max(1) as u32,
            (bounds.height - offset * 2).max(1) as u32,
        );
        draw_hollow_rect_mut(canvas, inset, color);
    }
    let _ = rect;
}

fn draw_label(canvas: &mut RgbImage, bounds: &crate::types::Bounds, text: &str, color: Rgb<u8>) {
    let Some(font) = load_font() else {
        return;
    };
    let label = truncate_label(text, 24);
    let scale = ab_glyph::PxScale::from(14.0);
    let x = bounds.x.max(0) as i32;
    let y = (bounds.y - 16).max(0) as i32;
    draw_text_mut(canvas, color, x, y, scale, &font, &label);
}

fn truncate_label(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    format!("{}…", trimmed.chars().take(max_chars).collect::<String>())
}

fn load_font() -> Option<ab_glyph::FontRef<'static>> {
    static FONT: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
    let data = FONT.get_or_init(|| {
        #[cfg(windows)]
        {
            for path in [
                "C:/Windows/Fonts/segoeui.ttf",
                "C:/Windows/Fonts/arial.ttf",
            ] {
                if let Ok(bytes) = std::fs::read(path) {
                    return Some(bytes);
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            if let Ok(bytes) = std::fs::read("/System/Library/Fonts/Supplemental/Arial.ttf") {
                return Some(bytes);
            }
        }
        #[cfg(target_os = "linux")]
        {
            for path in [
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/TTF/DejaVuSans.ttf",
            ] {
                if let Ok(bytes) = std::fs::read(path) {
                    return Some(bytes);
                }
            }
        }
        None
    });

    let bytes = data.as_ref()?;
    // Font data lives for program lifetime via OnceLock.
    let leaked: &'static [u8] = Box::leak(bytes.clone().into_boxed_slice());
    ab_glyph::FontRef::try_from_slice(leaked).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Bounds;

    #[test]
    fn renders_without_panic() {
        let img = DynamicImage::new_rgb8(100, 100);
        let result = ExtractResult {
            width: 100,
            height: 100,
            root: UiElement::container(
                Bounds::new(0, 0, 100, 100),
                vec![UiElement::text(
                    Bounds::new(10, 10, 40, 20),
                    "Hi".into(),
                    Some(90.0),
                )],
            ),
        };
        let out = render_annotation(&img, &result);
        assert_eq!(out.dimensions(), (100, 100));
    }
}
