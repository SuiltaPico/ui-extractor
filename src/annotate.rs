use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
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
    let scale = PxScale::from(14.0);
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

fn font_search_paths() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &[
            // Prefer CJK-capable fonts; Segoe UI / Arial lack most Han glyphs.
            "C:/Windows/Fonts/msyh.ttc",
            "C:/Windows/Fonts/msyhl.ttc",
            "C:/Windows/Fonts/simhei.ttf",
            "C:/Windows/Fonts/simsun.ttc",
            "C:/Windows/Fonts/DENG.TTF",
            "C:/Windows/Fonts/segoeui.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ]
    }
    #[cfg(target_os = "macos")]
    {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
        ]
    }
    #[cfg(not(any(windows, target_os = "macos", unix)))]
    {
        &[]
    }
}

fn font_can_render_cjk(bytes: &[u8]) -> bool {
    let Ok(font) = FontRef::try_from_slice(bytes) else {
        return false;
    };
    font_has_outline(&font, '中')
}

fn font_has_outline(font: &FontRef, ch: char) -> bool {
    let scale = PxScale::from(12.0);
    let scaled = font.as_scaled(scale);
    let glyph_id = scaled.glyph_id(ch);
    scaled
        .outline_glyph(glyph_id.with_scale_and_position(scale, point(0.0, 0.0)))
        .is_some()
}

fn read_best_font_bytes() -> Option<Vec<u8>> {
    let mut latin_fallback = None;
    for path in font_search_paths() {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if FontRef::try_from_slice(&bytes).is_err() {
            continue;
        }
        if font_can_render_cjk(&bytes) {
            return Some(bytes);
        }
        if latin_fallback.is_none() {
            latin_fallback = Some(bytes);
        }
    }
    latin_fallback
}

fn load_font() -> Option<FontRef<'static>> {
    static FONT_BYTES: std::sync::OnceLock<Option<&'static [u8]>> = std::sync::OnceLock::new();
    let bytes = (*FONT_BYTES.get_or_init(|| {
        read_best_font_bytes().map(|data| Box::leak(data.into_boxed_slice()) as &'static [u8])
    }))?;
    FontRef::try_from_slice(bytes).ok()
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

    #[test]
    fn annotation_font_renders_cjk_when_available() {
        let Some(font) = load_font() else {
            return;
        };
        assert!(
            font_has_outline(&font, '中'),
            "annotation font must include CJK glyphs (check font_search_paths)"
        );
    }
}
