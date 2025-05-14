use std::io::BufReader;

use eframe::emath::{Rect, Vec2};
use eframe::epaint::Color32;
use gerber_viewer::gerber_parser::parse;
use gerber_viewer::{draw_arrow, draw_outline, draw_crosshair, BoundingBox, GerberLayer, GerberRenderer, Transform2D, ViewState, Mirroring};
use gerber_viewer::position::{Position, Vector};

const ENABLE_UNIQUE_SHAPE_COLORS: bool = true;
const ENABLE_POLYGON_NUMBERING: bool = true;
const ZOOM_FACTOR: f32 = 0.5;
const ROTATION: f32 = 45.0_f32.to_radians();
const MIRRORING: [bool; 2] = [false, false];

// for mirroring and rotation
const CENTER_OFFSET: Vector = Vector::new(0.0, 0.0);

// in EDA tools like DipTrace, a gerber offset can be specified when exporting gerbers, e.g. 10,5.
// use negative offsets here to relocate the gerber back to 0,0, e.g. -10, -5
const DESIGN_OFFSET: Vector = Vector::new(0.0, 0.0);

struct DemoApp {
    gerber_layer: GerberLayer,
    view_state: ViewState,
    needs_initial_view: bool,

    // these are all in gerber coordinates (not screen coordinates)
    bbox: BoundingBox,
    bbox_vertices: Vec<Position>,
    outline_vertices: Vec<Position>,
}

impl DemoApp {
    pub fn new() -> Self {
        let demo_str = include_str!("../assets/demo.gbr").as_bytes();
        let reader = BufReader::new(demo_str);

        let doc = parse(reader).unwrap();
        let commands = doc.into_commands();

        let gerber_layer = GerberLayer::new(commands);

        let bbox = gerber_layer
            .bounding_box();

        let origin = CENTER_OFFSET - DESIGN_OFFSET;

        //let center_of_geometry = bbox.center();

        let transform = Transform2D {
            rotation_radians: ROTATION,
            mirroring: MIRRORING.into(),
            origin,
            offset: DESIGN_OFFSET,
        };

        // this must be done before the bbox is transformed.
        let outline_vertices = bbox.transform_vertices(transform);

        let bbox = bbox
            .apply_transform(transform);

        let bbox_vertices = bbox.vertices();

        Self {
            bbox,
            bbox_vertices,
            outline_vertices,
            gerber_layer,
            view_state: Default::default(),
            needs_initial_view: true,
        }
    }

    fn reset_view(&mut self, viewport: Rect) {
        let content_width = self.bbox.width();
        let content_height = self.bbox.height();

        // Calculate scale to fit the content (100% zoom)
        let scale = f32::min(
            viewport.width() / (content_width as f32),
            viewport.height() / (content_height as f32),
        );
        // 50% zoom
        let scale = scale * ZOOM_FACTOR;
        // adjust slightly to add a margin
        let scale = scale * 0.95;

        let center = self.bbox.center();

        // Offset from viewport center to place content in the center
        self.view_state.translation = Vec2::new(
            viewport.center().x - (center.x as f32 * scale),
            viewport.center().y + (center.y as f32 * scale), // Note the + here since we flip Y
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

                let gerber_zero_screen_position = self.view_state.gerber_to_screen_coords(Position::ZERO);
                draw_crosshair(&painter, gerber_zero_screen_position, Color32::BLUE);

                GerberRenderer::default().paint_layer(
                    &painter,
                    self.view_state,
                    &self.gerber_layer,
                    Color32::WHITE,
                    ENABLE_UNIQUE_SHAPE_COLORS,
                    ENABLE_POLYGON_NUMBERING,
                    ROTATION,
                    MIRRORING.into(),
                    CENTER_OFFSET.into(),
                    DESIGN_OFFSET.into(),
                );

                let design_offset_screen_position = self.view_state.gerber_to_screen_coords(DESIGN_OFFSET.to_position());
                draw_arrow(&painter, design_offset_screen_position, gerber_zero_screen_position, Color32::ORANGE);

                let bbox_vertices = self.bbox_vertices.iter().map(|position|{
                    self.view_state.gerber_to_screen_coords(position.clone())
                }).collect::<Vec<_>>();

                draw_outline(&painter, bbox_vertices, Color32::RED);

                let outline_vertices = self.outline_vertices.iter().map(|position|{
                    self.view_state.gerber_to_screen_coords(position.clone())
                }).collect::<Vec<_>>();

                draw_outline(&painter, outline_vertices, Color32::GREEN);
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
