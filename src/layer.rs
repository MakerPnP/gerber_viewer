use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use egui::Pos2;
#[cfg(feature = "egui")]
use egui::epaint::emath::Vec2;
use log::{debug, error, info, trace, warn};

use super::expressions::{
    ExpressionEvaluationError, MacroContext, evaluate_expression, macro_boolean_to_bool, macro_decimal_pair_to_f64,
    macro_decimal_to_f64, macro_integer_to_u32,
};
use super::geometry::{BoundingBox, PolygonMesh};
use super::gerber_types::{
    Aperture, ApertureDefinition, ApertureMacro, Command, Coordinates, DCode, ExtendedCode, FunctionCode, GCode,
    MacroContent, MacroDecimal, Operation, VariableDefinition,
};
use super::position::deduplicate::DedupEpsilon;
use super::{Exposure, Position, Winding};
use super::{calculate_winding, geometry, gerber_types};

pub struct GerberLayer {
    /// Storing the commands, soon we'll want to tag the primitives with the `Command` used to build them.
    #[allow(unused)]
    commands: Vec<Command>,
    gerber_primitives: Vec<GerberPrimitive>,
    bounding_box: BoundingBox,
}

impl GerberLayer {
    pub fn new(commands: Vec<Command>) -> Self {
        let gerber_primitives = GerberLayer::build_primitives(&commands);
        let bounding_box = GerberLayer::calculate_bounding_box(&gerber_primitives);

        Self {
            commands,
            gerber_primitives,
            bounding_box,
        }
    }

    /// It's possible to have a gerber file with no primitives
    pub fn is_empty(&self) -> bool {
        self.bounding_box.is_empty()
    }

    pub fn bounding_box(&self) -> &BoundingBox {
        &self.bounding_box
    }

    /// Return the bounding box if the gerber file resulted in primitives which need drawing.
    pub fn try_bounding_box(&self) -> Option<&BoundingBox> {
        match self.is_empty() {
            true => None,
            false => Some(&self.bounding_box),
        }
    }

    pub fn primitives(&self) -> &[GerberPrimitive] {
        &self.gerber_primitives
    }
}

impl GerberLayer {
    fn update_position(current_pos: &mut Position, coords: &Coordinates) {
        *current_pos = (
            coords
                .x
                .map(|value| value.into())
                .unwrap_or(current_pos.x),
            coords
                .y
                .map(|value| value.into())
                .unwrap_or(current_pos.y),
        )
            .into()
    }

    fn calculate_bounding_box(primitives: &Vec<GerberPrimitive>) -> BoundingBox {
        let mut bbox = BoundingBox::default();

        // Calculate bounding box
        for primitive in primitives {
            match primitive {
                GerberPrimitive::Circle {
                    center,
                    diameter,
                    ..
                } => {
                    let radius = diameter / 2.0;
                    bbox.min.x = bbox.min.x.min(center.x - radius);
                    bbox.min.y = bbox.min.y.min(center.y - radius);
                    bbox.max.x = bbox.max.x.max(center.x + radius);
                    bbox.max.y = bbox.max.y.max(center.y + radius);
                }
                GerberPrimitive::Rectangle {
                    origin,
                    width,
                    height,
                    ..
                } => {
                    bbox.min.x = bbox.min.x.min(origin.x);
                    bbox.min.y = bbox.min.y.min(origin.y);
                    bbox.max.x = bbox.max.x.max(origin.x + width);
                    bbox.max.y = bbox.max.y.max(origin.y + height);
                }
                GerberPrimitive::Line {
                    start,
                    end,
                    width,
                    ..
                } => {
                    let radius = width / 2.0;
                    for &Position {
                        x,
                        y,
                    } in &[start, end]
                    {
                        bbox.min.x = bbox.min.x.min(x - radius);
                        bbox.min.y = bbox.min.y.min(y - radius);
                        bbox.max.x = bbox.max.x.max(x + radius);
                        bbox.max.y = bbox.max.y.max(y + radius);
                    }
                }
                GerberPrimitive::Polygon {
                    center,
                    geometry,
                    ..
                } => {
                    for &Position {
                        x: dx,
                        y: dy,
                    } in geometry.relative_vertices.iter()
                    {
                        let x = center.x + dx;
                        let y = center.y + dy;
                        bbox.min.x = bbox.min.x.min(x);
                        bbox.min.y = bbox.min.y.min(y);
                        bbox.max.x = bbox.max.x.max(x);
                        bbox.max.y = bbox.max.y.max(y);
                    }
                }
            }
        }

        trace!("layer bbox: {:?}", bbox);

        bbox
    }

    fn build_primitives(commands: &[Command]) -> Vec<GerberPrimitive> {
        let mut macro_definitions: HashMap<String, &ApertureMacro> = HashMap::default();

        // First pass: collect aperture macros
        for cmd in commands.iter() {
            if let Command::ExtendedCode(ExtendedCode::ApertureMacro(macro_def)) = cmd {
                macro_definitions.insert(macro_def.name.clone(), macro_def);
            }
        }

        // Second pass - collect aperture definitions, build their primitives (using supplied args)

        let mut apertures: HashMap<i32, ApertureKind> = HashMap::default();

        for cmd in commands.iter() {
            if let Command::ExtendedCode(ExtendedCode::ApertureDefinition(ApertureDefinition {
                code,
                aperture,
            })) = cmd
            {
                match aperture {
                    Aperture::Macro(macro_name, args) => {
                        // Handle macro-based apertures

                        if let Some(macro_def) = macro_definitions.get(macro_name) {
                            //
                            // build a unique name based on the macro name and args
                            //
                            let macro_name_and_args = match args {
                                None => macro_name,
                                Some(args) => {
                                    let args_str = args
                                        .iter()
                                        .map(|arg| {
                                            let meh = match arg {
                                                MacroDecimal::Value(value) => value.to_string(),
                                                MacroDecimal::Variable(variable) => format!("${}", variable),
                                                MacroDecimal::Expression(expression) => expression.clone(),
                                            };

                                            meh
                                        })
                                        .collect::<Vec<_>>()
                                        .join("X");

                                    &format!("{}_{}", macro_name, args_str)
                                }
                            };
                            debug!("macro_name_and_args: {}", macro_name_and_args);

                            let mut macro_context = MacroContext::default();

                            //
                            // populate the macro_context from the args.
                            //
                            if let Some(args) = args {
                                for (index, arg) in args.iter().enumerate() {
                                    let arg_number = (index + 1) as u32;

                                    match arg {
                                        MacroDecimal::Value(value) => {
                                            macro_context
                                                .put(arg_number, *value)
                                                .inspect_err(|error| {
                                                    error!("Error setting variable {}: {}", arg_number, error);
                                                })
                                                .ok();
                                        }
                                        MacroDecimal::Variable(variable) => {
                                            macro_context
                                                .put(arg_number, macro_context.get(variable))
                                                .inspect_err(|error| {
                                                    error!("Error setting variable {}: {}", arg_number, error);
                                                })
                                                .ok();
                                        }
                                        MacroDecimal::Expression(expression) => {
                                            evaluate_expression(&expression, &macro_context)
                                                .map(|value| {
                                                    macro_context
                                                        .put(arg_number, value)
                                                        .inspect_err(|error| {
                                                            error!("Error setting variable {}: {}", arg_number, error);
                                                        })
                                                        .ok();
                                                })
                                                .inspect_err(|error| {
                                                    error!("Error evaluating expression {}: {}", expression, error);
                                                })
                                                .ok();
                                        }
                                    }
                                }
                            }

                            trace!("initial macro_context: {:?}", macro_context);

                            let mut primitive_defs = vec![];

                            for content in &macro_def.content {
                                trace!("macro_content: {:?}", content);

                                fn process_content(
                                    content: &MacroContent,
                                    macro_context: &mut MacroContext,
                                ) -> Result<Option<GerberPrimitive>, ExpressionEvaluationError>
                                {
                                    match content {
                                        MacroContent::Circle(circle) => {
                                            let diameter = macro_decimal_to_f64(&circle.diameter, macro_context)?;
                                            let (center_x, center_y) =
                                                macro_decimal_pair_to_f64(&circle.center, macro_context)?;

                                            // Get rotation angle and convert to radians
                                            let rotation_radians = if let Some(angle) = &circle.angle {
                                                macro_decimal_to_f64(angle, macro_context)? * std::f64::consts::PI
                                                    / 180.0
                                            } else {
                                                0.0
                                            };

                                            // Apply rotation to center coordinates around macro origin (0,0)
                                            let (sin_theta, cos_theta) = rotation_radians.sin_cos();
                                            let rotated_x = center_x * cos_theta - center_y * sin_theta;
                                            let rotated_y = center_x * sin_theta + center_y * cos_theta;

                                            Ok(Some(GerberPrimitive::Circle {
                                                center: (rotated_x, rotated_y).into(),
                                                diameter,
                                                exposure: macro_boolean_to_bool(&circle.exposure, macro_context)?
                                                    .into(),
                                            }))
                                        }
                                        MacroContent::VectorLine(vector_line) => {
                                            // Get parameters
                                            let (start_x, start_y) =
                                                macro_decimal_pair_to_f64(&vector_line.start, macro_context)?;
                                            let (end_x, end_y) =
                                                macro_decimal_pair_to_f64(&vector_line.end, macro_context)?;
                                            let width = macro_decimal_to_f64(&vector_line.width, macro_context)?;
                                            let rotation_angle =
                                                macro_decimal_to_f64(&vector_line.angle, macro_context)?;
                                            let rotation_radians = rotation_angle.to_radians();
                                            let (sin_theta, cos_theta) = rotation_radians.sin_cos();

                                            // Rotate start and end points
                                            let rotated_start_x = start_x * cos_theta - start_y * sin_theta;
                                            let rotated_start_y = start_x * sin_theta + start_y * cos_theta;
                                            let rotated_end_x = end_x * cos_theta - end_y * sin_theta;
                                            let rotated_end_y = end_x * sin_theta + end_y * cos_theta;

                                            // Calculate direction vector
                                            let dx = rotated_end_x - rotated_start_x;
                                            let dy = rotated_end_y - rotated_start_y;
                                            let length = (dx * dx + dy * dy).sqrt();

                                            if length == 0.0 {
                                                return Ok(None);
                                            }

                                            // Calculate perpendicular direction
                                            let ux = dx / length;
                                            let uy = dy / length;
                                            let perp_x = -uy;
                                            let perp_y = ux;

                                            // Calculate width offsets
                                            let half_width = width / 2.0;
                                            let hw_perp_x = perp_x * half_width;
                                            let hw_perp_y = perp_y * half_width;

                                            // Calculate corners in absolute coordinates
                                            let corners = [
                                                (rotated_start_x - hw_perp_x, rotated_start_y - hw_perp_y),
                                                (rotated_start_x + hw_perp_x, rotated_start_y + hw_perp_y),
                                                (rotated_end_x + hw_perp_x, rotated_end_y + hw_perp_y),
                                                (rotated_end_x - hw_perp_x, rotated_end_y - hw_perp_y),
                                            ];

                                            // Calculate center point
                                            let center_x = (rotated_start_x + rotated_end_x) / 2.0;
                                            let center_y = (rotated_start_y + rotated_end_y) / 2.0;

                                            // Convert to relative vertices
                                            let vertices = corners
                                                .iter()
                                                .map(|&(x, y)| Position::new(x - center_x, y - center_y))
                                                .collect();

                                            Ok(Some(GerberPrimitive::new_polygon(GerberPolygon {
                                                center: Position::new(center_x, center_y),
                                                vertices,
                                                exposure: macro_boolean_to_bool(&vector_line.exposure, macro_context)?
                                                    .into(),
                                            })))
                                        }
                                        MacroContent::CenterLine(center_line) => {
                                            // Get parameters
                                            let (center_x, center_y) =
                                                macro_decimal_pair_to_f64(&center_line.center, macro_context)?;
                                            let (length, width) =
                                                macro_decimal_pair_to_f64(&center_line.dimensions, macro_context)?;
                                            let rotation_angle =
                                                macro_decimal_to_f64(&center_line.angle, macro_context)?;
                                            let rotation_radians = rotation_angle.to_radians();
                                            let (sin_theta, cos_theta) = rotation_radians.sin_cos();

                                            // Calculate half dimensions
                                            let half_length = length / 2.0;
                                            let half_width = width / 2.0;

                                            // Define unrotated vertices relative to center
                                            let unrotated_vertices = [
                                                Position::new(half_length, half_width),
                                                Position::new(-half_length, half_width),
                                                Position::new(-half_length, -half_width),
                                                Position::new(half_length, -half_width),
                                            ];

                                            // Rotate each vertex relative to the center
                                            let vertices = unrotated_vertices
                                                .iter()
                                                .map(|pos| {
                                                    let x = pos.x * cos_theta - pos.y * sin_theta;
                                                    let y = pos.x * sin_theta + pos.y * cos_theta;
                                                    Position::new(x, y)
                                                })
                                                .collect();

                                            Ok(Some(GerberPrimitive::new_polygon(GerberPolygon {
                                                center: Position::new(center_x, center_y),
                                                vertices,
                                                exposure: macro_boolean_to_bool(&center_line.exposure, macro_context)?
                                                    .into(),
                                            })))
                                        }
                                        MacroContent::Outline(outline) => {
                                            // Need at least 3 points to form a polygon
                                            if outline.points.len() < 3 {
                                                warn!("Outline with less than 3 points. outline: {:?}", outline);
                                                return Ok(None);
                                            }

                                            // Get vertices - points are already relative to (0,0)
                                            let mut vertices: Vec<Position> = outline
                                                .points
                                                .iter()
                                                .filter_map(|point| {
                                                    macro_decimal_pair_to_f64(point, macro_context)
                                                        .map(|d| d.into())
                                                        .inspect_err(|err| {
                                                            error!("Error building vertex: {}", err);
                                                        })
                                                        .ok()
                                                })
                                                .collect::<Vec<_>>();

                                            // Get rotation angle and convert to radians
                                            let rotation_degrees = macro_decimal_to_f64(&outline.angle, macro_context)?;
                                            let rotation_radians = rotation_degrees * std::f64::consts::PI / 180.0;

                                            // If there's rotation, apply it to all vertices around (0,0)
                                            if rotation_radians != 0.0 {
                                                let (sin_theta, cos_theta) = rotation_radians.sin_cos();
                                                vertices = vertices
                                                    .into_iter()
                                                    .map(
                                                        |Position {
                                                             x,
                                                             y,
                                                         }| {
                                                            let rotated_x = x * cos_theta - y * sin_theta;
                                                            let rotated_y = x * sin_theta + y * cos_theta;
                                                            (rotated_x, rotated_y).into()
                                                        },
                                                    )
                                                    .collect();
                                            }

                                            Ok(Some(GerberPrimitive::new_polygon(GerberPolygon {
                                                center: (0.0, 0.0).into(), // The flash operation will move this to final position
                                                vertices,
                                                exposure: macro_boolean_to_bool(&outline.exposure, macro_context)?
                                                    .into(),
                                            })))
                                        }
                                        MacroContent::Polygon(polygon) => {
                                            let center = macro_decimal_pair_to_f64(&polygon.center, macro_context)?;

                                            let vertices_count =
                                                macro_integer_to_u32(&polygon.vertices, macro_context)? as usize;
                                            let diameter = macro_decimal_to_f64(&polygon.diameter, macro_context)?;
                                            let rotation_degrees = macro_decimal_to_f64(&polygon.angle, macro_context)?;
                                            let rotation_radians = rotation_degrees * std::f64::consts::PI / 180.0;

                                            // First generate vertices around (0,0)
                                            let radius = diameter / 2.0;
                                            let mut vertices = Vec::with_capacity(vertices_count);
                                            for i in 0..vertices_count {
                                                let angle =
                                                    (2.0 * std::f64::consts::PI * i as f64) / vertices_count as f64;
                                                let x = radius * angle.cos();
                                                let y = radius * angle.sin();

                                                // Apply rotation around macro origin (0,0)
                                                let (sin_theta, cos_theta) = rotation_radians.sin_cos();
                                                let rotated_x = x * cos_theta - y * sin_theta;
                                                let rotated_y = x * sin_theta + y * cos_theta;

                                                vertices.push((rotated_x, rotated_y).into());
                                            }

                                            // Rotate center point around macro origin
                                            let (sin_theta, cos_theta) = rotation_radians.sin_cos();
                                            let rotated_center_x = center.0 * cos_theta - center.1 * sin_theta;
                                            let rotated_center_y = center.0 * sin_theta + center.1 * cos_theta;

                                            Ok(Some(GerberPrimitive::new_polygon(GerberPolygon {
                                                center: (rotated_center_x, rotated_center_y).into(),
                                                vertices,
                                                exposure: macro_boolean_to_bool(&polygon.exposure, macro_context)?
                                                    .into(),
                                            })))
                                        }
                                        MacroContent::Moire(_) => {
                                            error!("Moire not supported");
                                            Ok(None)
                                        }
                                        MacroContent::Thermal(_) => {
                                            error!("Moire not supported");
                                            Ok(None)
                                        }
                                        MacroContent::VariableDefinition(VariableDefinition {
                                            number,
                                            expression,
                                        }) => {
                                            let result = evaluate_expression(&expression, macro_context);
                                            match result {
                                                Ok(value) => {
                                                    macro_context
                                                        .put(*number, value)
                                                        .inspect_err(|error| {
                                                            error!("Error setting variable {}: {}", number, error);
                                                        })
                                                        .ok();
                                                }
                                                Err(cause) => {
                                                    error!("Error evaluating expression {}: {}", expression, cause);
                                                }
                                            };
                                            Ok(None)
                                        }
                                        MacroContent::Comment(_) => {
                                            // Nothing to do
                                            Ok(None)
                                        }
                                    }
                                }

                                let result = process_content(content, &mut macro_context);
                                match result {
                                    Err(cause) => {
                                        error!("Error processing macro content: {:?}, cause: {}", content, cause);
                                    }
                                    Ok(Some(primitive)) => primitive_defs.push(primitive),
                                    Ok(None) => {}
                                }
                            }
                            trace!("final macro_context: {:?}", macro_context);

                            trace!("primitive_defs: {:?}", primitive_defs);

                            apertures.insert(*code, ApertureKind::Macro(primitive_defs));
                        } else {
                            error!(
                                "Aperture definition references unknown macro. macro_name: {}",
                                macro_name
                            );
                        }
                    }
                    _ => {
                        apertures.insert(*code, ApertureKind::Standard(aperture.clone()));
                    }
                }
            }
        }
        info!("macros: {:?}", macro_definitions.len());

        debug!("aperture codes: {:?}", apertures.keys());
        info!("apertures: {:?}", apertures.len());

        // Third pass: collect all primitives, handle regions

        let mut layer_primitives = Vec::new();
        let mut current_aperture = None;
        let mut current_pos = Position::ZERO;

        // also record aperture selection errors
        let mut aperture_selection_errors: HashSet<i32> = HashSet::new();

        // regions are a special case - they are defined by aperture codes
        let mut current_region_vertices: Vec<Position> = Vec::new();
        let mut in_region = false;

        for cmd in commands.iter() {
            match cmd {
                Command::FunctionCode(FunctionCode::GCode(GCode::RegionMode(enabled))) => {
                    if *enabled {
                        // G36 - Begin Region
                        in_region = true;
                        current_region_vertices.clear();
                    } else {
                        // G37 - End Region
                        if in_region && current_region_vertices.len() >= 3 {
                            // Find bounding box
                            let min_x = current_region_vertices
                                .iter()
                                .map(
                                    |Position {
                                         x, ..
                                     }| *x,
                                )
                                .fold(f64::INFINITY, f64::min);
                            let max_x = current_region_vertices
                                .iter()
                                .map(
                                    |Position {
                                         x, ..
                                     }| *x,
                                )
                                .fold(f64::NEG_INFINITY, f64::max);
                            let min_y = current_region_vertices
                                .iter()
                                .map(
                                    |Position {
                                         y, ..
                                     }| *y,
                                )
                                .fold(f64::INFINITY, f64::min);
                            let max_y = current_region_vertices
                                .iter()
                                .map(
                                    |Position {
                                         y, ..
                                     }| *y,
                                )
                                .fold(f64::NEG_INFINITY, f64::max);

                            // Calculate center from bounding box
                            let center_x = (min_x + max_x) / 2.0;
                            let center_y = (min_y + max_y) / 2.0;

                            let center = Position::new(center_x, center_y);

                            // Make vertices relative to center
                            let relative_vertices: Vec<Position> = current_region_vertices
                                .iter()
                                .map(|position| *position - center)
                                .collect();

                            let polygon = GerberPrimitive::new_polygon(GerberPolygon {
                                center: (center_x, center_y).into(),
                                vertices: relative_vertices,
                                exposure: Exposure::Add,
                            });
                            layer_primitives.push(polygon);
                            in_region = false;
                        }
                    }
                }

                Command::FunctionCode(FunctionCode::DCode(DCode::SelectAperture(code))) => {
                    current_aperture = apertures.get(&code);
                    if current_aperture.is_none() {
                        aperture_selection_errors.insert(*code);
                    }
                }
                Command::FunctionCode(FunctionCode::DCode(DCode::Operation(operation))) => {
                    match operation {
                        Operation::Move(coords) => {
                            let mut end = current_pos;
                            Self::update_position(&mut end, coords);
                            if in_region {
                                // In a region, a move operation starts a new path segment
                                // If we already have vertices, close the current segment
                                if !current_region_vertices.is_empty() {
                                    current_region_vertices.push(*current_region_vertices.first().unwrap());
                                }
                                // Start new segment
                                //current_region_vertices.push(end);
                            }
                            current_pos = end;
                        }
                        Operation::Interpolate(coords, ..) => {
                            let mut end = current_pos;
                            Self::update_position(&mut end, coords);
                            if in_region {
                                // Add vertex to current region
                                current_region_vertices.push(end);
                            } else if let Some(aperture) = current_aperture {
                                match aperture {
                                    ApertureKind::Standard(Aperture::Circle(gerber_types::Circle {
                                        diameter,
                                        ..
                                    })) => {
                                        layer_primitives.push(GerberPrimitive::Line {
                                            start: current_pos,
                                            end,
                                            width: *diameter,
                                            exposure: Exposure::Add,
                                        });
                                    }
                                    _ => {
                                        // TODO support more Apertures (rectangle, obround, etc)
                                    }
                                }
                            }
                            current_pos = end;
                        }
                        Operation::Flash(coords, ..) => {
                            if in_region {
                                warn!("Flash operation found within region - ignoring");
                            } else {
                                Self::update_position(&mut current_pos, coords);

                                if let Some(aperture) = current_aperture {
                                    match aperture {
                                        ApertureKind::Macro(macro_primitives) => {
                                            for primitive in macro_primitives {
                                                let mut primitive = primitive.clone();
                                                // Update the primitive's position based on flash coordinates
                                                match &mut primitive {
                                                    GerberPrimitive::Polygon {
                                                        center, ..
                                                    } => {
                                                        *center += current_pos;
                                                    }
                                                    GerberPrimitive::Circle {
                                                        center, ..
                                                    } => {
                                                        *center += current_pos;
                                                    }
                                                    GerberPrimitive::Rectangle {
                                                        origin, ..
                                                    } => {
                                                        *origin += current_pos;
                                                    }
                                                    GerberPrimitive::Line {
                                                        start,
                                                        end,
                                                        ..
                                                    } => {
                                                        *start += current_pos;
                                                        *end += current_pos;
                                                    }
                                                }
                                                trace!("flashing macro primitive: {:?}", primitive);
                                                layer_primitives.push(primitive);
                                            }
                                        }
                                        ApertureKind::Standard(aperture) => {
                                            match aperture {
                                                Aperture::Circle(circle) => {
                                                    layer_primitives.push(GerberPrimitive::Circle {
                                                        center: current_pos,
                                                        diameter: circle.diameter,
                                                        exposure: Exposure::Add,
                                                    });
                                                }
                                                Aperture::Rectangle(rect) => {
                                                    layer_primitives.push(GerberPrimitive::Rectangle {
                                                        origin: Position::new(
                                                            current_pos.x - rect.x / 2.0,
                                                            current_pos.y - rect.y / 2.0,
                                                        ),
                                                        width: rect.x,
                                                        height: rect.y,
                                                        exposure: Exposure::Add,
                                                    });
                                                }
                                                Aperture::Polygon(polygon) => {
                                                    let radius = polygon.diameter / 2.0;
                                                    let vertices_count = polygon.vertices as usize;
                                                    let mut vertices = Vec::with_capacity(vertices_count);

                                                    // For standard aperture polygon, we need to generate vertices
                                                    // starting at angle 0 and moving counterclockwise
                                                    for i in 0..vertices_count {
                                                        let angle = (2.0 * std::f64::consts::PI * i as f64)
                                                            / vertices_count as f64;
                                                        let x = radius * angle.cos();
                                                        let y = radius * angle.sin();

                                                        // Apply rotation if specified
                                                        let final_position = if let Some(rotation) = polygon.rotation {
                                                            let rot_rad = rotation * std::f64::consts::PI / 180.0;
                                                            let (sin_rot, cos_rot) = rot_rad.sin_cos();
                                                            (x * cos_rot - y * sin_rot, x * sin_rot + y * cos_rot)
                                                                .into()
                                                        } else {
                                                            (x, y).into()
                                                        };

                                                        vertices.push(final_position);
                                                    }

                                                    layer_primitives.push(GerberPrimitive::new_polygon(
                                                        GerberPolygon {
                                                            center: current_pos,
                                                            vertices,
                                                            exposure: Exposure::Add,
                                                        },
                                                    ));
                                                }
                                                Aperture::Obround(rect) => {
                                                    // For an obround, we need to:
                                                    // 1. Create a rectangle for the center part
                                                    // 2. Add two circles (one at each end)
                                                    // The longer dimension determines which way the semicircles go

                                                    let (rect_width, rect_height, circle_centers) = if rect.x > rect.y {
                                                        // Horizontal obround
                                                        let rect_width = rect.x - rect.y; // Subtract circle diameter
                                                        let circle_offset = rect_width / 2.0;
                                                        (rect_width, rect.y, [
                                                            (circle_offset, 0.0),
                                                            (-circle_offset, 0.0),
                                                        ])
                                                    } else {
                                                        // Vertical obround
                                                        let rect_height = rect.y - rect.x; // Subtract circle diameter
                                                        let circle_offset = rect_height / 2.0;
                                                        (rect.x, rect_height, [
                                                            (0.0, circle_offset),
                                                            (0.0, -circle_offset),
                                                        ])
                                                    };

                                                    // Add the center rectangle
                                                    layer_primitives.push(GerberPrimitive::Rectangle {
                                                        origin: Position::new(
                                                            current_pos.x - rect_width / 2.0,
                                                            current_pos.y - rect_height / 2.0,
                                                        ),
                                                        width: rect_width,
                                                        height: rect_height,
                                                        exposure: Exposure::Add,
                                                    });

                                                    // Add the end circles
                                                    let circle_radius = rect.x.min(rect.y) / 2.0;
                                                    for (dx, dy) in circle_centers {
                                                        layer_primitives.push(GerberPrimitive::Circle {
                                                            center: current_pos + Position::from((dx, dy)),
                                                            diameter: circle_radius * 2.0,
                                                            exposure: Exposure::Add,
                                                        });
                                                    }
                                                }
                                                Aperture::Macro(code, _args) => {
                                                    // if the aperture referred to a macro, and the macro was supported, it will have been handled by the `ApertureKind::Macro` handling.
                                                    warn!("Unsupported macro aperture: {:?}, code: {}", aperture, code);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if aperture_selection_errors.len() > 0 {
            error!(
                "Selecting some apertures failed; Check gerber file content and parser errors. aperture_codes: {:?}",
                aperture_selection_errors
            );
        }

        info!("layer_primitives: {:?}", layer_primitives.len());

        layer_primitives
    }
}

#[derive(Debug)]
enum ApertureKind {
    Standard(Aperture),
    Macro(Vec<GerberPrimitive>),
}

#[derive(Debug, Clone)]
pub enum GerberPrimitive {
    Circle {
        center: Position,
        diameter: f64,
        exposure: Exposure,
    },
    Rectangle {
        origin: Position,
        width: f64,
        height: f64,
        exposure: Exposure,
    },
    Line {
        start: Position,
        end: Position,
        width: f64,
        exposure: Exposure,
    },
    Polygon {
        center: Position,
        exposure: Exposure,
        geometry: Arc<PolygonGeometry>,
    },
}

#[derive(Debug, Clone)]
pub struct PolygonGeometry {
    pub relative_vertices: Vec<Position>,  // Relative to center
    pub tessellation: Option<PolygonMesh>, // Precomputed tessellation data
    pub is_convex: bool,
}

#[derive(Debug)]
pub struct GerberPolygon {
    center: Position,
    /// Relative to center
    vertices: Vec<Position>,
    exposure: Exposure,
}

impl GerberPolygon {
    /// Checks if a polygon is convex by verifying that all cross products
    /// between consecutive edges have the same sign
    pub fn is_convex(&self) -> bool {
        geometry::is_convex(&self.vertices)
    }
}

impl GerberPrimitive {
    fn new_polygon(polygon: GerberPolygon) -> Self {
        trace!("new_polygon: {:?}", polygon);
        let is_convex = polygon.is_convex();
        let mut relative_vertices = polygon.vertices;

        // Calculate and fix winding order
        let winding = calculate_winding(&relative_vertices);
        if matches!(winding, Winding::Clockwise) {
            relative_vertices.reverse();
        }

        // Deduplicate adjacent vertices with geometric tolerance
        let epsilon = 1e-6; // 1 nanometer in mm units
        let relative_vertices = relative_vertices.dedup_with_epsilon(epsilon);

        // Precompute tessellation for concave polygons
        let tessellation = if !is_convex {
            Some(geometry::tessellate_polygon(&relative_vertices))
        } else {
            None
        };

        let polygon = GerberPrimitive::Polygon {
            center: polygon.center,
            exposure: polygon.exposure,
            geometry: Arc::new(PolygonGeometry {
                relative_vertices,
                tessellation,
                is_convex,
            }),
        };

        trace!("polygon: {:?}", polygon);

        polygon
    }
}

#[cfg(feature = "egui")]
#[derive(Debug, Copy, Clone)]
pub struct ViewState {
    pub translation: Vec2,
    pub scale: f32,
}

#[cfg(feature = "egui")]
impl Default for ViewState {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            scale: 1.0,
        }
    }
}

impl ViewState {
    /// Convert to gerber coordinates using view transformation
    pub fn screen_to_gerber_coords(&self, screen_pos: Pos2) -> Position {
        let gerber_pos = (screen_pos - self.translation) / self.scale;
        Position::new(gerber_pos.x as f64, gerber_pos.y as f64).invert_y()
    }

    /// Convert from gerber coordinates using view transformation
    pub fn gerber_to_screen_coords(&self, gerber_pos: Position) -> Pos2 {
        let gerber_pos = gerber_pos.invert_y();
        ((gerber_pos * self.scale as f64) + self.translation).to_pos2()
    }
}
