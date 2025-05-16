use std::io::BufReader;

use eframe::emath::{Rect, Vec2};
use eframe::epaint::Color32;
use egui::ViewportBuilder;
use gerber_viewer::gerber_parser::parse;
use gerber_viewer::{draw_arrow, draw_outline, draw_crosshair, BoundingBox, GerberLayer, GerberRenderer, Transform2D, ViewState, Mirroring, draw_marker};
use gerber_viewer::position::{Position, Vector};

const ENABLE_UNIQUE_SHAPE_COLORS: bool = true;
const ENABLE_POLYGON_NUMBERING: bool = true;
const ZOOM_FACTOR: f32 = 0.50;
const ROTATION_SPEED_DEG_PER_SEC: f32 = 45.0;
const INITIAL_ROTATION: f32 = 45.0_f32.to_radians();
const MIRRORING: [bool; 2] = [false, false];

// for mirroring and rotation
const CENTER_OFFSET: Vector = Vector::new(15.0, 20.0);
//const CENTER_OFFSET: Vector = Vector::new(14.75, 6.0);

// in EDA tools like DipTrace, a gerber offset can be specified when exporting gerbers, e.g. 10,5.
// use negative offsets here to relocate the gerber back to 0,0, e.g. -10, -5
const DESIGN_OFFSET: Vector = Vector::new(-5.0, -10.0);
//const DESIGN_OFFSET: Vector = Vector::new(-10.0, -10.0);

// radius of the markers, in gerber coordinates
const MARKER_RADIUS: f32 = 2.5;

struct DemoApp {
    gerber_layer: GerberLayer,
    view_state: ViewState,
    needs_initial_view: bool,

    last_frame_time: std::time::Instant,
    rotation_radians: f32
}

impl DemoApp {
    pub fn new() -> Self {
        let demo_str = include_str!("../assets/demo.gbr").as_bytes();
        //let demo_str = include_str!("../assets/rectangles.gbr").as_bytes();
        //let demo_str = include_str!("../assets/macro-vectorline.gbr").as_bytes();
        //let demo_str = include_str!("../assets/macro-polygons.gbr").as_bytes();
        //let demo_str = include_str!("../assets/macro-polygons-concave.gbr").as_bytes();

        //let demo_str = include_str!(r#"D:\Users\Hydra\Documents\DipTrace\Projects\SPRacingRXN1\Export\SPRacingRXN1-RevB-20240507-1510_gerberx2\TopSilk.gbr"#).as_bytes();


        let reader = BufReader::new(demo_str);

        let doc = parse(reader).unwrap();
        let commands = doc.into_commands();

        let gerber_layer = GerberLayer::new(commands);

        Self {
            last_frame_time: std::time::Instant::now(),
            gerber_layer,
            view_state: Default::default(),
            needs_initial_view: true,
            rotation_radians: INITIAL_ROTATION,
        }
    }

    fn reset_view(&mut self, viewport: Rect) {
        let bbox = self.gerber_layer.bounding_box();
        let content_width = bbox.width();
        let content_height = bbox.height();

        // Calculate scale to fit the content (100% zoom)
        let scale = f32::min(
            viewport.width() / (content_width as f32),
            viewport.height() / (content_height as f32),
        );
        // 50% zoom
        let scale = scale * ZOOM_FACTOR;
        // adjust slightly to add a margin
        let scale = scale * 0.95;

        let center = bbox.center();

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

        //
        // Animate the gerber view by rotating it.
        //
        let now = std::time::Instant::now();
        let delta = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        let rotation_increment = ROTATION_SPEED_DEG_PER_SEC.to_radians() * delta;
        self.rotation_radians += rotation_increment;

        if ROTATION_SPEED_DEG_PER_SEC > 0.0 {
            // force the UI to refresh every frame for a smooth animation
            ctx.request_repaint();
        }

        //
        // Compute bounding box and outline
        //

        let bbox = self.gerber_layer.bounding_box();

        let origin = CENTER_OFFSET - DESIGN_OFFSET;

        let transform = Transform2D {
            rotation_radians: self.rotation_radians,
            mirroring: MIRRORING.into(),
            origin,
            offset: DESIGN_OFFSET,
        };

        // Compute rotated outline (GREEN)
        let outline_vertices: Vec<_> = bbox
            .vertices()
            .into_iter()
            .map(|v| transform.apply_to_position(v))
            .collect();

        // Compute transformed AABB (RED)
        let bbox = BoundingBox::from_points(&outline_vertices);

        // Convert to screen coords
        let bbox_vertices_screen = bbox.vertices().into_iter()
            .map(|v| self.view_state.gerber_to_screen_coords(v))
            .collect::<Vec<_>>();

        let outline_vertices_screen = outline_vertices.into_iter()
            .map(|v| self.view_state.gerber_to_screen_coords(v))
            .collect::<Vec<_>>();


        //
        // Build a UI
        //
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                let response = ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::empty());
                let viewport = response.rect;

                if self.needs_initial_view {
                    self.reset_view(viewport)
                }

                //
                // Show the gerber layer and other overlays
                //

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
                    self.rotation_radians,
                    MIRRORING.into(),
                    CENTER_OFFSET.into(),
                    DESIGN_OFFSET.into(),
                );

                draw_outline(&painter, bbox_vertices_screen, Color32::RED);
                draw_outline(&painter, outline_vertices_screen, Color32::GREEN);

                let screen_radius = MARKER_RADIUS * self.view_state.scale;

                let design_offset_screen_position = self.view_state.gerber_to_screen_coords(DESIGN_OFFSET.to_position());
                draw_arrow(&painter, design_offset_screen_position, gerber_zero_screen_position, Color32::ORANGE);
                draw_marker(&painter, design_offset_screen_position, Color32::ORANGE, Color32::YELLOW, screen_radius);

                let design_origin_screen_position = self.view_state.gerber_to_screen_coords((CENTER_OFFSET - DESIGN_OFFSET).to_position());
                draw_marker(&painter, design_origin_screen_position, Color32::PURPLE, Color32::MAGENTA, screen_radius);
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (optional).
    eframe::run_native(
        "Gerber Viewer Demo (egui)",
        eframe::NativeOptions {
            viewport: ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(DemoApp::new()))),
    )
}
