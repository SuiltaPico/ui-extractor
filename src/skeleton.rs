use std::path::Path;

use crate::error::{ExtractError, Result};
use crate::types::{ExtractResult, UiElement, UiElementKind};

const DEPTH_COLORS: &[&str] = &[
    "#e63946", // 1
    "#457b9d", // 2
    "#2a9d8f", // 3
    "#e9c46a", // 4
    "#9b5de5", // 5
    "#f72585", // 6
    "#fb8500", // 7
    "#06d6a0", // 8
];

/// Render an interactive wireframe HTML page from an extract result.
pub fn render_skeleton_html(result: &ExtractResult, title: Option<&str>) -> String {
    let page_title = title.unwrap_or("UI Skeleton");
    let mut nodes = String::new();
    render_element(&mut nodes, &result.root, 0);

    let legend: String = DEPTH_COLORS
        .iter()
        .enumerate()
        .map(|(i, color)| {
            format!(
                r#"<span class="legend-item"><i style="background:{color}"></i>L{}</span>"#,
                i + 1
            )
        })
        .collect();

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{page_title}</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: "Segoe UI", system-ui, sans-serif;
      background: #0d1117;
      color: #c9d1d9;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 20px;
      gap: 12px;
    }}
    header {{
      width: min(100%, {width}px);
      display: flex;
      flex-wrap: wrap;
      align-items: baseline;
      justify-content: space-between;
      gap: 8px;
    }}
    h1 {{ font-size: 1rem; font-weight: 600; color: #f0f6fc; }}
    .meta {{ font-size: 0.85rem; color: #8b949e; }}
    .legend {{
      display: flex;
      flex-wrap: wrap;
      gap: 10px;
      font-size: 0.75rem;
      color: #8b949e;
    }}
    .legend-item {{ display: inline-flex; align-items: center; gap: 4px; }}
    .legend-item i {{
      display: inline-block;
      width: 14px;
      height: 14px;
      border-radius: 2px;
      border: 2px solid currentColor;
    }}
    .canvas-wrap {{
      overflow: auto;
      max-width: 100%;
      border: 1px solid #30363d;
      border-radius: 6px;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    }}
    .canvas {{
      position: relative;
      background: #161b22;
      background-image:
        linear-gradient(rgba(255,255,255,0.03) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255,255,255,0.03) 1px, transparent 1px);
      background-size: 20px 20px;
    }}
    .node {{
      position: absolute;
      border: 2px solid;
      background: rgba(255, 255, 255, 0.04);
      overflow: hidden;
      pointer-events: auto;
      transition: background 0.15s, box-shadow 0.15s;
    }}
    .node:hover {{
      background: rgba(255, 255, 255, 0.1);
      box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.2);
      z-index: 9999;
    }}
    .node.kind-text {{
      border-style: dashed;
      background: rgba(0, 0, 0, 0.25);
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 2px;
    }}
    .badge {{
      position: absolute;
      top: 0;
      left: 0;
      font-size: 9px;
      line-height: 1;
      padding: 2px 4px;
      background: rgba(0, 0, 0, 0.55);
      color: #fff;
      border-bottom-right-radius: 4px;
      pointer-events: none;
      white-space: nowrap;
    }}
    .text-content {{
      font-size: var(--fs, 11px);
      color: rgba(255, 255, 255, 0.92);
      text-align: center;
      line-height: 1.2;
      word-break: break-all;
      pointer-events: none;
    }}
  </style>
</head>
<body>
  <header>
    <h1>{page_title}</h1>
    <span class="meta">{width} × {height} px</span>
  </header>
  <div class="legend">{legend}<span class="legend-item"><i style="border-style:dashed;background:transparent"></i>text</span></div>
  <div class="canvas-wrap">
    <div class="canvas" style="width:{width}px;height:{height}px">
{nodes}    </div>
  </div>
</body>
</html>
"#,
        page_title = html_escape(page_title),
        width = result.width,
        height = result.height,
        legend = legend,
        nodes = nodes,
    )
}

pub fn write_skeleton_html(path: &Path, result: &ExtractResult) -> Result<()> {
    let title = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("UI Skeleton");
    let html = render_skeleton_html(result, Some(title));
    std::fs::write(path, html).map_err(|e| ExtractError::Image(e.to_string()))
}

fn render_element(out: &mut String, element: &UiElement, depth: usize) {
    if depth > 0 {
        append_node(out, element, depth);
    }

    for child in &element.children {
        render_element(out, child, depth + 1);
    }
}

fn append_node(out: &mut String, element: &UiElement, depth: usize) {
    let b = &element.bounds;
    let color = depth_color(depth);
    let badge = match &element.kind {
        UiElementKind::Root => return,
        UiElementKind::Container => format!("L{depth} · container"),
        UiElementKind::Text { .. } => format!("L{depth} · text"),
        UiElementKind::Icon { .. } => format!("L{depth} · icon"),
    };

    let (kind_class, title_attr, inner) = match &element.kind {
        UiElementKind::Root => return,
        UiElementKind::Container => (String::from("kind-container"), String::new(), String::new()),
        UiElementKind::Text { content, confidence } => {
            let conf = confidence
                .map(|c| format!(" · {:.0}%", c))
                .unwrap_or_default();
            let fs = text_font_size(b.height);
            (
                String::from("kind-text"),
                format!(r#" title="{}{conf}""#, html_escape(content)),
                format!(
                    r#"<span class="text-content" style="--fs:{fs}px">{content}</span>"#,
                    fs = fs,
                    content = html_escape(content),
                ),
            )
        }
        UiElementKind::Icon { name, confidence } => {
            let conf = confidence
                .map(|c| format!(" · {:.0}%", c * 100.0))
                .unwrap_or_default();
            (
                String::from("kind-icon"),
                format!(r#" title="{}{conf}""#, html_escape(name)),
                format!(
                    r#"<span class="text-content" style="--fs:{fs}px">{name}</span>"#,
                    fs = text_font_size(b.height),
                    name = html_escape(name),
                ),
            )
        }
    };

    out.push_str(&format!(
        r#"      <div class="node {kind_class}" style="left:{x}px;top:{y}px;width:{w}px;height:{h}px;border-color:{color}"{title_attr}><span class="badge">{badge}</span>{inner}</div>
"#,
        kind_class = kind_class,
        x = b.x,
        y = b.y,
        w = b.width,
        h = b.height,
        color = color,
        title_attr = title_attr,
        badge = html_escape(&badge),
        inner = inner,
    ));
}

fn depth_color(depth: usize) -> &'static str {
    DEPTH_COLORS[(depth - 1) % DEPTH_COLORS.len()]
}

fn text_font_size(height: i32) -> i32 {
    (height * 2 / 3).clamp(8, 16)
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Bounds;

    #[test]
    fn renders_html_with_depth_colors() {
        let result = ExtractResult {
            width: 200,
            height: 100,
            root: UiElement::container(
                Bounds::new(0, 0, 200, 100),
                vec![
                    UiElement::container(
                        Bounds::new(10, 10, 80, 40),
                        vec![UiElement::text(
                            Bounds::new(20, 20, 40, 20),
                            "Hi".into(),
                            Some(90.0),
                        )],
                    ),
                    UiElement::text(Bounds::new(100, 10, 50, 20), "Btn".into(), None),
                ],
            ),
        };

        let html = render_skeleton_html(&result, Some("test"));
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("border-color:#e63946"));
        assert!(html.contains("border-color:#457b9d"));
        assert!(html.contains("Hi"));
    }

    #[test]
    fn write_skeleton_from_arknights_terminal_fixture() {
        let case_dir = Path::new("tests/cases/arknights-terminal");
        let json_path = case_dir.join("output.json");
        if !json_path.is_file() {
            return;
        }

        let json = std::fs::read_to_string(&json_path).unwrap();
        let result: ExtractResult = serde_json::from_str(&json).unwrap();
        write_skeleton_html(&case_dir.join("skeleton.html"), &result).unwrap();

        let html = std::fs::read_to_string(case_dir.join("skeleton.html")).unwrap();
        assert!(html.contains("arknights-terminal"));
        assert!(html.contains("相变临界"));
    }
}
