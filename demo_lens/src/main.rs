use std::io::BufReader;

use eframe::emath::{Rect, Vec2};
use eframe::epaint::Color32;
use egui::ViewportBuilder;
use egui_lens::{ReactiveEventLogger, ReactiveEventLoggerState, LogColors};
use egui_mobius_reactive::Dynamic;
use gerber_viewer::gerber_parser::parse;
use gerber_viewer::{
    draw_arrow, draw_outline, draw_crosshair, BoundingBox, GerberLayer, GerberRenderer, 
    Transform2D, ViewState, Mirroring, draw_marker, UiState
};
use gerber_viewer::position::{Position, Vector};

const ENABLE_UNIQUE_SHAPE_COLORS: bool = true;
const ENABLE_POLYGON_NUMBERING: bool = false;
const ZOOM_FACTOR: f32 = 0.50;
const ROTATION_SPEED_DEG_PER_SEC: f32 = 45.0;
const INITIAL_ROTATION: f32 = 45.0_f32.to_radians();
const MIRRORING: [bool; 2] = [false, false];

// for mirroring and rotation
const CENTER_OFFSET: Vector = Vector::new(15.0, 20.0);

// in EDA tools like DipTrace, a gerber offset can be specified when exporting gerbers, e.g. 10,5.
// use negative offsets here to relocate the gerber back to 0,0, e.g. -10, -5
const DESIGN_OFFSET: Vector = Vector::new(-5.0, -10.0);

// radius of the markers, in gerber coordinates
const MARKER_RADIUS: f32 = 2.5;

struct DemoLensApp {
    gerber_layer: GerberLayer,
    view_state: ViewState,
    ui_state: UiState,
    needs_initial_view: bool,

    last_frame_time: std::time::Instant,
    rotation_radians: f32,
    
    // Logger state
    logger_state: Dynamic<ReactiveEventLoggerState>,
    log_colors: Dynamic<LogColors>,
    
    // Properties
    rotation_speed: f32,
    zoom: f32,
    enable_unique_colors: bool,
    enable_polygon_numbering: bool,
    mirroring: Mirroring,
    center_offset: Vector,
    design_offset: Vector,
}

impl DemoLensApp {
    pub fn new() -> Self {
        let demo_str = include_str!("../assets/demo.gbr").as_bytes();

        let reader = BufReader::new(demo_str);

        let doc = parse(reader).unwrap();
        let commands = doc.into_commands();

        let gerber_layer = GerberLayer::new(commands);
        
        // Create logger state
        let logger_state = Dynamic::new(ReactiveEventLoggerState::new());
        let log_colors = Dynamic::new(LogColors::default());

        Self {
            last_frame_time: std::time::Instant::now(),
            gerber_layer,
            view_state: Default::default(),
            needs_initial_view: true,
            rotation_radians: INITIAL_ROTATION,
            ui_state: Default::default(),
            
            // Logger state
            logger_state,
            log_colors,
            
            // Properties with defaults
            rotation_speed: ROTATION_SPEED_DEG_PER_SEC,
            zoom: ZOOM_FACTOR,
            enable_unique_colors: ENABLE_UNIQUE_SHAPE_COLORS,
            enable_polygon_numbering: ENABLE_POLYGON_NUMBERING,
            mirroring: MIRRORING.into(),
            center_offset: CENTER_OFFSET,
            design_offset: DESIGN_OFFSET,
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
        // Use zoom from lens properties
        let scale = scale * self.zoom;
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

impl eframe::App for DemoLensApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //
        // Animate the gerber view by rotating it.
        //
        let now = std::time::Instant::now();
        let delta = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        let rotation_increment = self.rotation_speed.to_radians() * delta;
        self.rotation_radians += rotation_increment;

        if self.rotation_speed > 0.0 {
            // force the UI to refresh every frame for a smooth animation
            ctx.request_repaint();
        }

        //
        // Compute bounding box and outline
        //
        let bbox = self.gerber_layer.bounding_box();

        let origin = self.center_offset - self.design_offset;

        let transform = Transform2D {
            rotation_radians: self.rotation_radians,
            mirroring: self.mirroring,
            origin,
            offset: self.design_offset,
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
        
        // Left panel for property editing with egui
        egui::SidePanel::left("properties_panel").show(ctx, |ui| {
            ui.heading("Gerber Properties");
            ui.separator();
            
            // Create a logger for this frame
            let logger = ReactiveEventLogger::with_colors(&self.logger_state, &self.log_colors);
            
            // Regular egui widgets (without lens)
            ui.label("Rotation Speed (deg/s)");
            if ui.add(egui::Slider::new(&mut self.rotation_speed, 0.0..=180.0)).changed() {
                logger.log_info(&format!("Rotation speed changed to {:.1} deg/s", self.rotation_speed));
            }
            
            ui.label("Zoom Factor");
            ui.add(egui::Slider::new(&mut self.zoom, 0.1..=2.0));
            if ui.button("Apply Zoom").clicked() {
                self.needs_initial_view = true;
                logger.log_info(&format!("Zoom reset to {:.2}", self.zoom));
            }
            
            ui.checkbox(&mut self.enable_unique_colors, "Enable Unique Colors");
            ui.checkbox(&mut self.enable_polygon_numbering, "Enable Polygon Numbering");
            
            ui.separator();
            ui.heading("Mirroring");
            ui.checkbox(&mut self.mirroring.x, "X Mirror");
            ui.checkbox(&mut self.mirroring.y, "Y Mirror");
            
            ui.separator();
            ui.heading("Center Offset");
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut self.center_offset.x).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut self.center_offset.y).speed(0.1));
            });
            
            ui.separator();
            ui.heading("Design Offset");
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut self.design_offset.x).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut self.design_offset.y).speed(0.1));
            });
            
            ui.separator();
            ui.heading("Event Log");
            // Display the logger
            logger.show(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                let response = ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::drag());
                let viewport = response.rect;

                if self.needs_initial_view {
                    self.reset_view(viewport)
                }
                
                //
                // handle pan, drag and cursor position
                //
                self.ui_state.update(ui, &viewport, &response, &mut self.view_state);

                //
                // Show the gerber layer and other overlays
                //

                let painter = ui.painter().with_clip_rect(viewport);
                
                draw_crosshair(&painter, self.ui_state.origin_screen_pos, Color32::BLUE);
                draw_crosshair(&painter, self.ui_state.center_screen_pos, Color32::LIGHT_GRAY);

                GerberRenderer::default().paint_layer(
                    &painter,
                    self.view_state,
                    &self.gerber_layer,
                    Color32::WHITE,
                    self.enable_unique_colors,
                    self.enable_polygon_numbering,
                    self.rotation_radians,
                    self.mirroring,
                    self.center_offset.into(),
                    self.design_offset.into(),
                );
                
                // if you want to display multiple layers, call `paint_layer` for each layer. 

                draw_outline(&painter, bbox_vertices_screen, Color32::RED);
                draw_outline(&painter, outline_vertices_screen, Color32::GREEN);

                let screen_radius = MARKER_RADIUS * self.view_state.scale;

                let design_offset_screen_position = self.view_state.gerber_to_screen_coords(self.design_offset.to_position());
                draw_arrow(&painter, design_offset_screen_position, self.ui_state.origin_screen_pos, Color32::ORANGE);
                draw_marker(&painter, design_offset_screen_position, Color32::ORANGE, Color32::YELLOW, screen_radius);

                let design_origin_screen_position = self.view_state.gerber_to_screen_coords((self.center_offset - self.design_offset).to_position());
                draw_marker(&painter, design_origin_screen_position, Color32::PURPLE, Color32::MAGENTA, screen_radius);
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (optional).
    eframe::run_native(
        "Gerber Viewer Lens Demo (egui)",
        eframe::NativeOptions {
            viewport: ViewportBuilder::default().with_inner_size([1280.0, 768.0]),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(DemoLensApp::new()))),
    )
}