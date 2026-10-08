use crate::AppWindow;
use slint::ComponentHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl VideoGeometry {
    pub fn from_slint(ui: &AppWindow) -> Option<Self> {
        let scale = ui.window().scale_factor();

        let x = ui.get_video_x();
        let y = ui.get_video_y();
        let width = ui.get_video_width();
        let height = ui.get_video_height();

        if !scale.is_finite()
            || scale <= 0.0
            || !x.is_finite()
            || !y.is_finite()
            || !width.is_finite()
            || !height.is_finite()
            || width <= 0.0
            || height <= 0.0
        {
            return None;
        }

        // Logical pixels -> physical pixels.
        let x = (x * scale).round() as i32;
        let y = (y * scale).round() as i32;
        let width = (width * scale).round() as i32;
        let height = (height * scale).round() as i32;

        if width <= 0 || height <= 0 {
            return None;
        }

        Some(Self {
            x,
            y,
            width,
            height,
        })
    }
}
