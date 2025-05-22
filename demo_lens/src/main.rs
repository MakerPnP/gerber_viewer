use std::io::BufReader;

use eframe::emath::{Rect, Vec2};
use eframe::epaint::Color32;
use egui::ViewportBuilder;
use egui_lens::{ReactiveEventLogger, ReactiveEventLoggerState, LogColors};
use egui_mobius_reactive::*; 
use gerber_viewer::gerber_parser::parse;
use gerber_viewer::{
    draw_arrow, draw_outline, draw_crosshair, BoundingBox, GerberLayer, GerberRenderer, 
    Transform2D, ViewState, Mirroring, draw_marker, UiState
};
use gerber_viewer::position::Vector;


// Import platform modules
mod platform;
use platform::{banner, details, parameters::gui};

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
    banner: banner::Banner,
    details: details::Details,
    
    // Properties
    rotation_speed: f32,
    zoom: f32,
    enable_unique_colors: bool,
    enable_polygon_numbering: bool,
    mirroring: Mirroring,
    center_offset: Vector,
    design_offset: Vector,
    
    // DRC Properties
    current_drc_ruleset: Option<String>,
}

impl DemoLensApp {
    // Custom log types for different event categories
    const LOG_TYPE_ROTATION: &'static str = "rotation";
    const LOG_TYPE_ZOOM: &'static str = "zoom";
    const LOG_TYPE_CENTER_OFFSET: &'static str = "center_offset";
    const LOG_TYPE_DESIGN_OFFSET: &'static str = "design_offset";
    const LOG_TYPE_MIRROR: &'static str = "mirror";
    const LOG_TYPE_DISPLAY: &'static str = "display";
    const LOG_TYPE_DRC: &'static str = "drc";
    
    /// Configure custom colors 
    /// 
    /// This function will get the current colors from the `Dynamic<LogColors>` instance, 
    /// check if the custom colors for the specified log types are already set,
    /// and if not, set them to the default values.
    ///
    fn configure_custom_log_colors_if_missing(colors: &mut Dynamic<LogColors>) {

        let mut colors_value = colors.get();
        
        if !colors_value.custom_colors.contains_key(Self::LOG_TYPE_ROTATION) {
            colors_value.set_custom_color(Self::LOG_TYPE_ROTATION, egui::Color32::from_rgb(230, 126, 34));
        }
        if !colors_value.custom_colors.contains_key(Self::LOG_TYPE_ZOOM) {
            colors_value.set_custom_color(Self::LOG_TYPE_ZOOM, egui::Color32::from_rgb(41, 128, 185));
        }
        if !colors_value.custom_colors.contains_key(Self::LOG_TYPE_CENTER_OFFSET) {
            colors_value.set_custom_color(Self::LOG_TYPE_CENTER_OFFSET, egui::Color32::from_rgb(142, 68, 173));
        }
        if !colors_value.custom_colors.contains_key(Self::LOG_TYPE_DESIGN_OFFSET) {
            colors_value.set_custom_color(Self::LOG_TYPE_DESIGN_OFFSET, egui::Color32::from_rgb(39, 174, 96));
        }
        if !colors_value.custom_colors.contains_key(Self::LOG_TYPE_MIRROR) {
            colors_value.set_custom_color(Self::LOG_TYPE_MIRROR, egui::Color32::from_rgb(192, 57, 43));
        }
        if !colors_value.custom_colors.contains_key(Self::LOG_TYPE_DISPLAY) {
            colors_value.set_custom_color(Self::LOG_TYPE_DISPLAY, egui::Color32::from_rgb(241, 196, 15));
        }
        if !colors_value.custom_colors.contains_key(Self::LOG_TYPE_DRC) {
            colors_value.set_custom_color(Self::LOG_TYPE_DRC, egui::Color32::from_rgb(155, 89, 182));
        }
        
        colors.set(colors_value);
    }
    
    /// **Color change watcher** 
    /// 
    /// This function sets up a watcher for changes to the log colors, and when a change is detected,
    /// it saves the current colors to a JSON file in the config directory. The egui_mobius_reactive 
    /// on_change method is used to trigger the save operation whenever the colors change. One does not
    /// need to call the signal registery for the `Derived` type to do this, as the `on_change` method 
    /// is a built in to the `Dynamic` type.
    ///
    fn watch_for_color_changes(&self) {
        let log_colors_clone = self.log_colors.clone();
        
        self.log_colors.on_change(move || {
            let colors = log_colors_clone.get();
            
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("gerber_viewer");
            
            if let Err(e) = std::fs::create_dir_all(&config_dir) {
                eprintln!("Failed to create config directory: {}", e);
                return;
            }
            
            let config_path = config_dir.join("log_colors.json");
            
            match serde_json::to_string_pretty(&colors) {
                Ok(json) => {
                    // Write JSON to file
                    if let Err(e) = std::fs::write(&config_path, json) {
                        eprintln!("Failed to write colors to {}: {}", config_path.display(), e);
                    } else {
                        println!("Successfully saved colors to {}", config_path.display());
                    }
                },
                Err(e) => eprintln!("Failed to serialize colors: {}", e),
            }
        });
    }
    
    pub fn new() -> Self {
        let demo_str = include_str!("../assets/demo.gbr").as_bytes();

        let reader = BufReader::new(demo_str);

        let doc = parse(reader).unwrap();
        let commands = doc.into_commands();

        let gerber_layer = GerberLayer::new(commands);
        
        // Create logger state
        let logger_state = Dynamic::new(ReactiveEventLoggerState::new());
        
        // Custom load logic for gerber_viewer
        let mut log_colors = Dynamic::new({
            // Check if colors exist in the gerber_viewer config directory
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("gerber_viewer");
            let config_path = config_dir.join("log_colors.json");
            
            println!("Loading colors from: {}", config_path.display());
            
            if let Ok(file_content) = std::fs::read_to_string(&config_path) {
                match serde_json::from_str(&file_content) {
                    Ok(colors) => {
                        println!("Successfully loaded colors from file");
                        colors
                    }
                    Err(e) => {
                        eprintln!("Failed to parse colors JSON: {}", e);
                        LogColors::default()
                    }
                }
            } else {
                println!("No saved colors found, using defaults");
                LogColors::default()
            }
        });
        
        // Configure custom colors for different event types (only if they don't exist)
        Self::configure_custom_log_colors_if_missing(&mut log_colors);

         // Create banner and details instances
        let mut banner = banner::Banner::new();
        let mut details = details::Details::new();
        
        // Format banner and get system info
        banner.format();
        details.get_os();

        let app = Self {
            last_frame_time: std::time::Instant::now(),
            gerber_layer,
            view_state: Default::default(),
            needs_initial_view: true,
            rotation_radians: INITIAL_ROTATION,
            ui_state: Default::default(),
            
            // Logger state
            logger_state,
            log_colors,
            banner,
            details,
            
            // Properties with defaults
            rotation_speed: ROTATION_SPEED_DEG_PER_SEC,
            zoom: ZOOM_FACTOR,
            enable_unique_colors: ENABLE_UNIQUE_SHAPE_COLORS,
            enable_polygon_numbering: ENABLE_POLYGON_NUMBERING,
            mirroring: MIRRORING.into(),
            center_offset: CENTER_OFFSET,
            design_offset: DESIGN_OFFSET,
            
            // DRC Properties
            current_drc_ruleset: None,
        };
        
        // Setup color change watcher to auto-save when colors change
        app.watch_for_color_changes();

        app.add_banner_platform_details();
        
        app
    }

     fn add_banner_platform_details(&self) {
        // Create a logger using references to our Dynamic state
        let logger = ReactiveEventLogger::with_colors(&self.logger_state, &self.log_colors);
        
        // Log banner message (welcome message)
        logger.log_info(&self.banner.message);
        
        // Log system details
        let details_text = self.details.clone().format_os();
        logger.log_info(&details_text);
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
        // Create a logger for this frame
        let logger = ReactiveEventLogger::with_colors(&self.logger_state, &self.log_colors);
        
        let show_system_info = ctx.memory(|mem| {
            mem.data.get_temp::<bool>(egui::Id::new("show_system_info")).unwrap_or(false)
        });
        
        if show_system_info {
            // Clear the flag
            ctx.memory_mut(|mem| {
                mem.data.remove::<bool>(egui::Id::new("show_system_info"));
            });
            
            // Create a logger to display system info
            let logger = ReactiveEventLogger::with_colors(&self.logger_state, &self.log_colors);
            
            // Log system details
            let details_text = self.details.format_os();
            logger.log_info(&details_text);
            
            // Then log banner message
            logger.log_info(&self.banner.message);
        }
        
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
            
            // Regular egui widgets with custom event logging
            ui.label("Rotation Speed (deg/s)");
            let prev_rotation = self.rotation_speed;
            if ui.add(egui::Slider::new(&mut self.rotation_speed, 0.0..=180.0)).changed() {
                logger.log_custom(
                    Self::LOG_TYPE_ROTATION, 
                    &format!("Rotation speed changed from {:.1} to {:.1} deg/s", prev_rotation, self.rotation_speed)
                );
                
                // Save the colors whenever they're changed
                self.log_colors.get().save();
            }
            
            ui.label("Zoom Factor");
            let prev_zoom = self.zoom;
            if ui.add(egui::Slider::new(&mut self.zoom, 0.1..=2.0)).changed() {
                logger.log_custom(
                    Self::LOG_TYPE_ZOOM, 
                    &format!("Zoom factor changed from {:.2} to {:.2}", prev_zoom, self.zoom)
                );
            }
            if ui.button("Apply Zoom").clicked() {
                self.needs_initial_view = true;
                logger.log_custom(Self::LOG_TYPE_ZOOM, &format!("Zoom view reset with factor {:.2}", self.zoom));
            }
            
            let prev_unique_colors = self.enable_unique_colors;
            if ui.checkbox(&mut self.enable_unique_colors, "Enable Unique Colors").changed() {
                logger.log_custom(
                    Self::LOG_TYPE_DISPLAY, 
                    &format!("Unique colors {} for shapes", if self.enable_unique_colors { "enabled" } else { "disabled" })
                );
            }
            
            let prev_polygon_numbering = self.enable_polygon_numbering;
            if ui.checkbox(&mut self.enable_polygon_numbering, "Enable Polygon Numbering").changed() {
                logger.log_custom(
                    Self::LOG_TYPE_DISPLAY,
                    &format!("Polygon numbering {} for shapes", if self.enable_polygon_numbering { "enabled" } else { "disabled" })
                );
            }
            
            ui.separator();
            
            // Horizontal grid layout for Mirroring, Center Offset, and Design Offset
            ui.columns(3, |columns| {
                // Column 1: Mirroring
                columns[0].group(|ui| {
                    ui.heading("Mirroring");
                    ui.add_space(4.0);
                    
                    let prev_mirror_x = self.mirroring.x;
                    if ui.checkbox(&mut self.mirroring.x, "X Mirror").changed() {
                        logger.log_custom(
                            Self::LOG_TYPE_MIRROR,
                            &format!("X mirroring {}", if self.mirroring.x { "enabled" } else { "disabled" })
                        );
                        
                        // Save the colors whenever settings change
                        self.log_colors.get().save();
                    }
                    
                    let prev_mirror_y = self.mirroring.y;
                    if ui.checkbox(&mut self.mirroring.y, "Y Mirror").changed() {
                        logger.log_custom(
                            Self::LOG_TYPE_MIRROR,
                            &format!("Y mirroring {}", if self.mirroring.y { "enabled" } else { "disabled" })
                        );
                        
                        // Save the colors whenever settings change
                        self.log_colors.get().save();
                    }
                });
                
                // Column 2: Center Offset
                columns[1].group(|ui| {
                    ui.heading("Center Offset");
                    ui.add_space(4.0);
                    
                    let mut center_changed = false;
                    let old_center_x = self.center_offset.x;
                    let old_center_y = self.center_offset.y;
                    
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        if ui.add(egui::DragValue::new(&mut self.center_offset.x).speed(0.1)).changed() {
                            center_changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        if ui.add(egui::DragValue::new(&mut self.center_offset.y).speed(0.1)).changed() {
                            center_changed = true;
                        }
                    });
                    
                    if center_changed {
                        logger.log_custom(
                            Self::LOG_TYPE_CENTER_OFFSET,
                            &format!("Center offset changed from ({:.1}, {:.1}) to ({:.1}, {:.1})", 
                                    old_center_x, old_center_y, self.center_offset.x, self.center_offset.y)
                        );
                    }
                });
                
                // Column 3: Design Offset
                columns[2].group(|ui| {
                    ui.heading("Design Offset");
                    ui.add_space(4.0);
                    
                    let mut design_changed = false;
                    let old_design_x = self.design_offset.x;
                    let old_design_y = self.design_offset.y;
                    
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        if ui.add(egui::DragValue::new(&mut self.design_offset.x).speed(0.1)).changed() {
                            design_changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        if ui.add(egui::DragValue::new(&mut self.design_offset.y).speed(0.1)).changed() {
                            design_changed = true;
                        }
                    });
                    
                    if design_changed {
                        logger.log_custom(
                            Self::LOG_TYPE_DESIGN_OFFSET,
                            &format!("Design offset changed from ({:.1}, {:.1}) to ({:.1}, {:.1})", 
                                    old_design_x, old_design_y, self.design_offset.x, self.design_offset.y)
                        );
                    }
                });
            });
            
            ui.separator();
            
            // Design Rule Check section
            ui.horizontal(|ui| {
                ui.heading("Design Rule Check");
                
                // Add some spacing to push the button to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔍 Run DRC").clicked() {
                        // Check if a ruleset is loaded
                        if let Some(ref ruleset) = self.current_drc_ruleset {
                            // Simulate DRC process with INFO messages
                            logger.log_info("Starting Design Rule Check");
                            logger.log_info(&format!("Using {} ruleset", ruleset));
                            logger.log_info("Analyzing Gerber files");
                            logger.log_info("Checking trace widths");
                            logger.log_info("Checking via sizes");
                            logger.log_info("Checking spacing rules");
                            logger.log_info("Checking drill sizes");
                            logger.log_info("Issues found: None");
                            logger.log_info("DRC analysis completed successfully");
                        } else {
                            logger.log_warning("Cannot run DRC: No ruleset loaded");
                            logger.log_info("Please select a PCB manufacturer ruleset first");
                        }
                    }
                });
            });
            ui.add_space(4.0);
            
            egui::CollapsingHeader::new("PCB Manufacturer Rules")
                .default_open(false)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    
                    // Current ruleset display
                    if let Some(ref ruleset) = self.current_drc_ruleset {
                        ui.horizontal(|ui| {
                            ui.label("Current ruleset:");
                            ui.label(egui::RichText::new(ruleset).strong().color(egui::Color32::from_rgb(46, 204, 113)));
                        });
                        ui.add_space(4.0);
                    } else {
                        ui.label(egui::RichText::new("No DRC ruleset loaded").color(egui::Color32::from_rgb(231, 76, 60)));
                        ui.add_space(4.0);
                    }
                    
                    // PCB Manufacturer buttons
                    ui.vertical(|ui| {
                        if ui.button("🏭 JLC PCB Rules").clicked() {
                            self.current_drc_ruleset = Some("JLC PCB".to_string());
                            logger.log_custom(
                                Self::LOG_TYPE_DRC,
                                "Loaded JLC PCB Design Rule Check ruleset"
                            );
                        }
                        
                        if ui.button("🏭 PCB WAY Rules").clicked() {
                            self.current_drc_ruleset = Some("PCB WAY".to_string());
                            logger.log_custom(
                                Self::LOG_TYPE_DRC,
                                "Loaded PCB WAY Design Rule Check ruleset"
                            );
                        }
                        
                        if ui.button("🏭 Advanced Circuits Rules").clicked() {
                            self.current_drc_ruleset = Some("Advanced Circuits".to_string());
                            logger.log_custom(
                                Self::LOG_TYPE_DRC,
                                "Loaded Advanced Circuits Design Rule Check ruleset"
                            );
                        }
                        
                        ui.add_space(4.0);
                        
                        // Clear ruleset button
                        if self.current_drc_ruleset.is_some() {
                            if ui.button("🗑 Clear Ruleset").clicked() {
                                if let Some(ref ruleset) = self.current_drc_ruleset {
                                    logger.log_custom(
                                        Self::LOG_TYPE_DRC,
                                        &format!("Cleared {} Design Rule Check ruleset", ruleset)
                                    );
                                }
                                self.current_drc_ruleset = None;
                            }
                        }
                    });
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