use serde::{Deserialize, Serialize};

/// Axis-aligned bounding box in pixel coordinates (origin top-left).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Bounds {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }

    pub fn area(&self) -> i64 {
        self.width as i64 * self.height as i64
    }

    pub fn contains(&self, other: &Bounds) -> bool {
        self.x <= other.x
            && self.y <= other.y
            && self.right() >= other.right()
            && self.bottom() >= other.bottom()
    }

    pub fn intersection_area(&self, other: &Bounds) -> i64 {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());
        let w = (x2 - x1).max(0) as i64;
        let h = (y2 - y1).max(0) as i64;
        w * h
    }

    pub fn iou(&self, other: &Bounds) -> f64 {
        let inter = self.intersection_area(other) as f64;
        if inter <= 0.0 {
            return 0.0;
        }
        let union = self.area() as f64 + other.area() as f64 - inter;
        inter / union
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiElementKind {
    Root,
    Container,
    Text {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        confidence: Option<f32>,
    },
    Icon {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        confidence: Option<f32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    pub bounds: Bounds,
    #[serde(flatten)]
    pub kind: UiElementKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<UiElement>,
}

impl UiElement {
    pub fn container(bounds: Bounds, children: Vec<UiElement>) -> Self {
        Self {
            bounds,
            kind: UiElementKind::Container,
            children,
        }
    }

    pub fn text(bounds: Bounds, content: String, confidence: Option<f32>) -> Self {
        Self {
            bounds,
            kind: UiElementKind::Text { content, confidence },
            children: vec![],
        }
    }

    pub fn icon(bounds: Bounds, name: String, confidence: Option<f32>) -> Self {
        Self {
            bounds,
            kind: UiElementKind::Icon { name, confidence },
            children: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub width: i32,
    pub height: i32,
    pub root: UiElement,
}
