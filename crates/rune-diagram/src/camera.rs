/// Screen-space translation and a uniform scale, anchored at the content's top-left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 24.0,
            y: 24.0,
            scale: 1.0,
        }
    }
}

impl Camera {
    pub const MIN_SCALE: f32 = 0.1;
    pub const MAX_SCALE: f32 = 4.0;

    /// Fit with a 24-point margin. Small diagrams are not enlarged.
    pub fn fit(view: [f32; 2], content: [f32; 2]) -> Self {
        if !view
            .into_iter()
            .chain(content)
            .all(|v| v.is_finite() && v > 0.0)
        {
            return Self::default();
        }
        let scale = ((view[0] - 48.0) / content[0])
            .min((view[1] - 48.0) / content[1])
            .clamp(Self::MIN_SCALE, 1.0);
        Self {
            x: (view[0] - content[0] * scale) / 2.0,
            y: (view[1] - content[1] * scale) / 2.0,
            scale,
        }
    }

    /// Keep the content under `from` under `to` while zooming or pinching.
    pub fn zoom(&mut self, factor: f32, from: [f32; 2], to: [f32; 2]) {
        if !factor.is_finite() || factor <= 0.0 || !from.into_iter().chain(to).all(f32::is_finite) {
            return;
        }
        if !self.scale.is_finite() || self.scale <= 0.0 {
            *self = Self::default();
        }
        let scale = (self.scale * factor).clamp(Self::MIN_SCALE, Self::MAX_SCALE);
        self.x = to[0] - (from[0] - self.x) * scale / self.scale;
        self.y = to[1] - (from[1] - self.y) * scale / self.scale;
        self.scale = scale;
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        if dx.is_finite() && dy.is_finite() {
            self.x += dx;
            self.y += dy;
        }
    }
}
