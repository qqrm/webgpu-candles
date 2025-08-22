/// View parameters controlling zoom and pan.
#[derive(Clone, Debug)]
pub struct ViewState {
    pub pixels_per_candle: f32,
    pub pan_offset_px: f32,
    pub min_ppc: f32,
    pub max_ppc: f32,
    pub cursor_anchor_ratio: f32,
}

impl ViewState {
    pub fn new(pixels_per_candle: f32, min_ppc: f32, max_ppc: f32) -> Self {
        Self { pixels_per_candle, pan_offset_px: 0.0, min_ppc, max_ppc, cursor_anchor_ratio: 0.5 }
    }

    /// Zoom keeping the candle under the cursor stable.
    pub fn zoom_at(&mut self, delta_ppc: f32, cursor_ratio: f32, width_px: f32) {
        self.cursor_anchor_ratio = cursor_ratio;
        let cursor_px = width_px * cursor_ratio;
        let old_ppc = self.pixels_per_candle;
        let new_ppc = (old_ppc + delta_ppc).clamp(self.min_ppc, self.max_ppc);
        let candle_idx = (cursor_px - self.pan_offset_px) / old_ppc;
        self.pan_offset_px = cursor_px - candle_idx * new_ppc;
        self.pixels_per_candle = new_ppc;
    }

    /// Pan by pixel delta.
    pub fn pan(&mut self, delta_px: f32) {
        self.pan_offset_px += delta_px;
    }

    /// Visible candle range derived from view parameters.
    pub fn visible_range(&self, candle_count: usize, width_px: f32) -> (usize, usize) {
        let start = (-self.pan_offset_px / self.pixels_per_candle).floor().max(0.0) as usize;
        let visible = (width_px / self.pixels_per_candle).ceil() as usize;
        let start_clamped = start.min(candle_count.saturating_sub(visible));
        (start_clamped, visible.min(candle_count))
    }
}
