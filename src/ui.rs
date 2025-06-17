use egui::{Pos2, Rect, Response, Ui, Vec2};
use log::trace;
use nalgebra::Point2;

use crate::{BoundingBox, GerberTransform, Invert, ToPos2};

#[derive(Debug, Default)]
pub struct UiState {
    // these values are invalid until 'update' has been called
    pub center_screen_pos: Pos2,
    pub origin_screen_pos: Pos2,

    // only valid if the mouse is over the viewport
    pub cursor_gerber_coords: Option<Point2<f64>>,
}

impl UiState {
    pub fn update(&mut self, ui: &Ui, viewport: &Rect, response: &Response, view_state: &mut ViewState) {
        self.update_cursor_position(view_state, &response, ui);
        self.handle_panning(view_state, &response, ui);
        self.handle_zooming(view_state, &response, ui);

        self.center_screen_pos = viewport.center();
        self.origin_screen_pos = view_state.gerber_to_screen_coords(Point2::new(0.0, 0.0));

        trace!(
            "update. view_state: {:?}, viewport: {:?}, cursor_gerber_coords: {:?}",
            view_state,
            viewport,
            self.cursor_gerber_coords
        )
    }

    pub fn update_cursor_position(&mut self, view_state: &ViewState, response: &Response, ui: &Ui) {
        if !response.hovered() {
            return;
        }

        if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
            self.cursor_gerber_coords = Some(view_state.screen_to_gerber_coords(pointer_pos));
        } else {
            self.cursor_gerber_coords = None;
        }
    }

    pub fn handle_panning(&mut self, view_state: &mut ViewState, response: &Response, ui: &Ui) {
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            view_state.translation += delta;
            ui.ctx().clear_animations();
        }
    }

    pub fn handle_zooming(&mut self, view_state: &mut ViewState, response: &Response, ui: &Ui) {
        // Only process zoom if the mouse pointer is actually over the viewport
        if !response.hovered() {
            return;
        }

        let zoom_factor = 1.1;
        let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);

        if scroll_delta != 0.0 {
            let old_scale = view_state.scale;
            let new_scale = if scroll_delta > 0.0 {
                old_scale * zoom_factor
            } else {
                old_scale / zoom_factor
            };

            if let Some(hover_pos) = response.hover_pos() {
                let mouse_world = (hover_pos - view_state.translation) / old_scale;
                view_state.translation = hover_pos - mouse_world * new_scale;
            }

            view_state.scale = new_scale;
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ViewState {
    pub translation: Vec2,
    pub scale: f32,
    pub base_scale: f32, // Scale that represents 100% zoom
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            scale: 1.0,
            base_scale: 1.0,
        }
    }
}

impl ViewState {
    /// Convert to gerber coordinates using view transformation
    pub fn screen_to_gerber_coords(&self, screen_pos: Pos2) -> Point2<f64> {
        let gerber_pos = (screen_pos - self.translation) / self.scale;
        Point2::new(gerber_pos.x as f64, gerber_pos.y as f64).invert_y()
    }

    /// Convert from gerber coordinates using view transformation
    pub fn gerber_to_screen_coords(&self, gerber_pos: Point2<f64>) -> Pos2 {
        let gerber_pos = gerber_pos.invert_y();
        (gerber_pos * self.scale as f64).to_pos2() + self.translation
    }

    /// inputs, viewport of UI area to render.
    /// bounding box of all gerber layers to render.
    /// initial zoom factor, e.g. 0.5 for 50%.
    /// the initial transform.
    ///
    /// often you'll want to reset the `transform` before calling this.
    pub fn reset_view(
        &mut self,
        viewport: Rect,
        bbox: &BoundingBox,
        initial_zoom_factor: f32,
        transform: &GerberTransform,
    ) {
        let content_width = bbox.width();
        let content_height = bbox.height();

        // Calculate scale to fit the content (100% zoom)
        self.base_scale = f32::min(
            viewport.width() / (content_width as f32),
            viewport.height() / (content_height as f32),
        ) * 0.95; // 0.95 to add margin

        let scale = self.base_scale * initial_zoom_factor;

        // Compute transformed bounding box
        let outline_vertices: Vec<_> = bbox
            .vertices()
            .into_iter()
            .map(|v| transform.apply_to_position(v))
            .collect();

        let transformed_bbox = BoundingBox::from_points(&outline_vertices);

        // Use the center of the transformed bounding box
        let transformed_center = transformed_bbox.center();

        self.translation = Vec2::new(
            viewport.center().x - (transformed_center.x as f32 * scale),
            viewport.center().y + (transformed_center.y as f32 * scale),
        );

        self.scale = scale;
    }

    pub fn zoom_level_percent(&self) -> f32 {
        let zoom_level = self.scale / self.base_scale * 100.0;
        trace!("Zoom level: {:.1}%", zoom_level);

        zoom_level
    }

    pub fn set_zoom_level_percent(&mut self, zoom_level: f32) {
        self.scale = self.base_scale * zoom_level / 100.0;
    }
}
