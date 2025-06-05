use egui::{Pos2, Rect, Response, Ui};
use log::trace;
use nalgebra::Point2;

use crate::ViewState;

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
