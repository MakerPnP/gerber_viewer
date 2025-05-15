use std::io::BufReader;

use eframe::emath::{Rect, Vec2};
use eframe::epaint::Color32;
use gerber_viewer::gerber_parser::parse;
use gerber_viewer::{GerberLayer, GerberRenderer, ViewState};

struct DemoApp {
    gerber_layer: GerberLayer,
    view_state: ViewState,
    needs_initial_view: bool,
}

impl DemoApp {
    pub fn new() -> Self {
        let demo_str = include_str!("../assets/demo.gbr").as_bytes();
        let reader = BufReader::new(demo_str);

        let doc = parse(reader).unwrap();
        let commands = doc.into_commands();

        let gerber_layer = GerberLayer::new(commands);

        Self {
            gerber_layer,
            view_state: Default::default(),
            needs_initial_view: true,
        }
    }

    fn reset_view(&mut self, viewport: Rect) {
        let bbox = &self.gerber_layer.bounding_box();

        let content_width = bbox.max_x - bbox.min_x;
        let content_height = bbox.max_y - bbox.min_y;

        // Calculate scale to fit the content and adjust slightly to add a margin
        let scale = f32::min(
            viewport.width() / (content_width as f32),
            viewport.height() / (content_height as f32),
        ) * 0.95;

        // Calculate the content center (in gerber units)
        let content_center_x = (bbox.min_x + bbox.max_x) / 2.0;
        let content_center_y = (bbox.min_y + bbox.max_y) / 2.0;

        // Offset from viewport center to place content in the center
        self.view_state.translation = Vec2::new(
            viewport.center().x - (content_center_x as f32 * scale),
            viewport.center().y + (content_center_y as f32 * scale), // Note the + here since we flip Y
        );

        self.view_state.scale = scale;
        self.needs_initial_view = false;
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                let response = ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::empty());
                let viewport = response.rect;

                if self.needs_initial_view {
                    self.reset_view(viewport)
                }

                let painter = ui.painter().with_clip_rect(viewport);

                GerberRenderer::default().paint_layer(
                    &painter,
                    self.view_state,
                    &self.gerber_layer,
                    Color32::WHITE,
                    false,
                    false,
                );
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (optional).
    eframe::run_native(
        "Gerber Viewer Demo (egui)",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(DemoApp::new()))),
    )
}
