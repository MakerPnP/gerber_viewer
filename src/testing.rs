use std::io::BufWriter;

use gerber_types::{Command, GerberCode};

pub fn dump_gerber_source(commands: &Vec<Command>) {
    let gerber_source = gerber_commands_to_source(commands);

    println!("Gerber source:\n{}", gerber_source);
}

pub fn gerber_commands_to_source(commands: &Vec<Command>) -> String {
    let mut buf = BufWriter::new(Vec::new());
    commands
        .serialize(&mut buf)
        .expect("Could not generate Gerber code");
    let bytes = buf.into_inner().unwrap();
    let gerber_source = String::from_utf8(bytes).unwrap();
    gerber_source
}

pub mod geometry {
    use std::f64::consts::PI;

    /// generate points alternating between outer and inner radius
    pub fn calculate_alternating_points(
        outer_radius: f64,
        inner_radius: f64,
        center_x: f64,
        center_y: f64,
        sides: usize,
    ) -> Vec<(f64, f64)> {
        assert!(sides & 1 == 0, "Number of sides must be even");

        let mut points = Vec::new();
        let angle_step = (2.0 * PI) / sides as f64; // 36 degrees in radians

        for i in 0..sides {
            let radius = if i % 2 == 0 { outer_radius } else { inner_radius };
            let angle = angle_step * i as f64 - PI / 2.0;

            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();

            points.push((x, y));
        }
        points
    }

    pub fn extract_edges_and_midpoints(points: &[(f64, f64)]) -> (Vec<((f64, f64), (f64, f64))>, Vec<(f64, f64)>) {
        let len = points.len();
        assert!(len >= 3, "Need at least 3 points to form a closed shape");

        let mut edges = Vec::with_capacity(len);
        let mut midpoints = Vec::with_capacity(len);

        for i in 0..len {
            let a = points[i];
            let b = points[(i + 1) % len]; // wrap around to first point

            edges.push((a, b));
            midpoints.push(((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0));
        }

        (edges, midpoints)
    }

    pub fn compute_center_based_rotations(midpoints: &[(f64, f64)], shape_center: (f64, f64)) -> Vec<f64> {
        midpoints
            .iter()
            .map(|&(mx, my)| {
                let dx = mx - shape_center.0;
                let dy = my - shape_center.1;
                -dy.atan2(dx).to_degrees()
            })
            .collect()
    }

    pub fn compute_edge_rotations(edges: &[((f64, f64), (f64, f64))]) -> Vec<f64> {
        edges
            .iter()
            .map(|&(a, b)| {
                let dx = b.0 - a.0;
                let dy = b.1 - a.1;
                dy.atan2(dx)
                    .to_degrees()
                    .rem_euclid(360.0)
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const STAR5_POINTS: [(f64, f64); 10] = [
            (6.123233995736766e-17, -1.0),
            (0.29389262614623657, -0.4045084971874737),
            (0.9510565162951535, -0.3090169943749474),
            (0.47552825814757677, 0.1545084971874737),
            (0.5877852522924731, 0.8090169943749475),
            (3.061616997868383e-17, 0.5),
            (-0.587785252292473, 0.8090169943749475),
            (-0.47552825814757677, 0.15450849718747375),
            (-0.9510565162951536, -0.3090169943749473),
            (-0.2938926261462366, -0.40450849718747367),
        ];

        #[test]
        fn star_points() {
            // given
            let outer_diameter = 1.0;
            let inner_diameter = 0.5;

            let expected_points = Vec::from(STAR5_POINTS);
            // when
            let points = calculate_alternating_points(outer_diameter, inner_diameter, 0.0, 0.0, 5 * 2);

            // then
            assert_eq!(points, expected_points);
        }

        #[test]
        fn extract_edges_and_midpoints() {
            // given
            let points = Vec::from(STAR5_POINTS);

            // and
            // known-good output
            let expected_edges = vec![
                (
                    (6.123233995736766e-17, -1.0),
                    (0.29389262614623657, -0.4045084971874737),
                ),
                (
                    (0.29389262614623657, -0.4045084971874737),
                    (0.9510565162951535, -0.3090169943749474),
                ),
                (
                    (0.9510565162951535, -0.3090169943749474),
                    (0.47552825814757677, 0.1545084971874737),
                ),
                (
                    (0.47552825814757677, 0.1545084971874737),
                    (0.5877852522924731, 0.8090169943749475),
                ),
                ((0.5877852522924731, 0.8090169943749475), (3.061616997868383e-17, 0.5)),
                ((3.061616997868383e-17, 0.5), (-0.587785252292473, 0.8090169943749475)),
                (
                    (-0.587785252292473, 0.8090169943749475),
                    (-0.47552825814757677, 0.15450849718747375),
                ),
                (
                    (-0.47552825814757677, 0.15450849718747375),
                    (-0.9510565162951536, -0.3090169943749473),
                ),
                (
                    (-0.9510565162951536, -0.3090169943749473),
                    (-0.2938926261462366, -0.40450849718747367),
                ),
                (
                    (-0.2938926261462366, -0.40450849718747367),
                    (6.123233995736766e-17, -1.0),
                ),
            ];

            let expected_midpoints = vec![
                (0.1469463130731183, -0.7022542485937369),
                (0.622474571220695, -0.3567627457812106),
                (0.7132923872213651, -0.07725424859373685),
                (0.531656755220025, 0.4817627457812106),
                (0.29389262614623657, 0.6545084971874737),
                (-0.2938926261462365, 0.6545084971874737),
                (-0.5316567552200249, 0.4817627457812106),
                (-0.7132923872213652, -0.07725424859373677),
                (-0.6224745712206952, -0.3567627457812105),
                (-0.14694631307311828, -0.7022542485937369),
            ];

            // when
            let (edges, midpoints) = super::extract_edges_and_midpoints(&points);
            // then
            assert_eq!(edges, expected_edges);
            assert_eq!(midpoints, expected_midpoints);
        }
    }
}

mod macros {
    use gerber_types::{
        ApertureMacro, CenterLinePrimitive, CirclePrimitive, Command, ExtendedCode, MacroBoolean, MacroContent,
        MacroDecimal,
    };
    use log::trace;

    use crate::testing::geometry::{calculate_alternating_points, compute_edge_rotations, extract_edges_and_midpoints};

    /// used to generate code for demo gerber files
    #[allow(dead_code)]
    fn generate_star_outline_macro(outer_diameter: f64, inner_diameter: f64, name: String) -> Vec<Command> {
        generate_alternating_shape_outline_macro(outer_diameter, inner_diameter, 10, false, name)
    }

    /// used to generate code for demo gerber files
    ///
    /// midpoints are handy for debugging gerber primitive calculations and gerber rendering
    #[allow(dead_code)]
    fn generate_star_outline_macro_with_midpoints(
        outer_diameter: f64,
        inner_diameter: f64,
        name: String,
    ) -> Vec<Command> {
        generate_alternating_shape_outline_macro(outer_diameter, inner_diameter, 10, true, name)
    }

    /// used to generate code for demo gerber files
    #[allow(dead_code)]
    fn generate_diamond_outline_macro(outer_diameter: f64, inner_diameter: f64, name: String) -> Vec<Command> {
        generate_alternating_shape_outline_macro(outer_diameter, inner_diameter, 4, false, name)
    }

    /// used to generate code for demo gerber files
    ///
    /// midpoints are handy for debugging gerber primitive calculations and gerber rendering
    #[allow(dead_code)]
    fn generate_diamond_outline_macro_with_midpoints(
        outer_diameter: f64,
        inner_diameter: f64,
        name: String,
    ) -> Vec<Command> {
        generate_alternating_shape_outline_macro(outer_diameter, inner_diameter, 4, true, name)
    }

    fn generate_alternating_shape_outline_macro(
        outer_diameter: f64,
        inner_diameter: f64,
        sides: usize,
        with_midpoints: bool,
        name: String,
    ) -> Vec<Command> {
        let mut content = vec![
            MacroContent::Comment("$1 = outer diameter (scale)".to_string()),
            MacroContent::Comment("$2 = line width".to_string()),
            MacroContent::Comment("$3 = cos(rotation in degrees)".to_string()),
            MacroContent::Comment("$4 = sin(rotation in degrees)".to_string()),
            MacroContent::Comment("$5 = rotation in degrees".to_string()),
            MacroContent::Comment("gerber expressions do not support trigonometry functions like sin/cos, so they have to be pre-computed and supplied".to_string()),
        ];

        let center_x = 0.0;
        let center_y = 0.0;

        trace!("sides: {}", sides);
        trace!("outer_diameter: {}, inner_diameter: {}", outer_diameter, inner_diameter);
        trace!("center_x: {}, center_y: {}", center_x, center_y);
        trace!("outer_diameter: {}, inner_diameter: {}", outer_diameter, inner_diameter);
        trace!("center_x: {}, center_y: {}", center_x, center_y);

        content.push(MacroContent::Comment("end-points".to_string()));
        let points = calculate_alternating_points(outer_diameter, inner_diameter, center_x, center_y, sides);
        trace!("points: {:?}", points);

        let (edges, midpoints) = extract_edges_and_midpoints(&points);
        trace!("edges: {:?}", edges);
        trace!("midpoints: {:?}", midpoints);

        let edge_rotations = compute_edge_rotations(&edges);
        trace!("edge_rotations: {:?}", edge_rotations);

        for points in points.chunks_exact(2) {
            let build_end_point = |(x, y): &(f64, f64)| {
                let circle_item = MacroContent::Circle(CirclePrimitive {
                    exposure: MacroBoolean::Value(true),
                    diameter: MacroDecimal::Variable(2),
                    center: (
                        MacroDecimal::Expression(format!("$1x{:.6}", x)),
                        MacroDecimal::Expression(format!("$1x{:.6}", y)),
                    ),
                    angle: Some(MacroDecimal::Variable(5)),
                });
                circle_item
            };

            for point in points {
                content.push(build_end_point(point));
            }
        }

        if with_midpoints {
            content.push(MacroContent::Comment("mid-points".to_string()));
            for (mid_x, mid_y) in midpoints.iter() {
                let circle_item = MacroContent::Circle(CirclePrimitive {
                    exposure: MacroBoolean::Value(true),
                    diameter: MacroDecimal::Expression("$2x1.5".to_string()),
                    center: (
                        MacroDecimal::Expression(format!("$1x{:.6}", mid_x)),
                        MacroDecimal::Expression(format!("$1x{:.6}", mid_y)),
                    ),
                    angle: Some(MacroDecimal::Variable(5)),
                });
                content.push(circle_item);
            }
        }

        content.push(MacroContent::Comment("center-lines".to_string()));
        for ((((x1, y1), (x2, y2)), (mid_x, mid_y)), rotation) in edges
            .iter()
            .zip(midpoints.iter())
            .zip(edge_rotations.iter())
        {
            let dx = x2 - x1;
            let dy = y2 - y1;
            let length = (dx * dx + dy * dy).sqrt();

            let angle_rad = dy.atan2(dx);
            let angle_deg = angle_rad.to_degrees();
            println!(
                "line: dx: {}, dy: {}, length: {}, angle (old): {}, rotation (new): {}",
                dx, dy, length, angle_deg, rotation
            );

            let circle_item = MacroContent::CenterLine(CenterLinePrimitive {
                exposure: MacroBoolean::Value(true),
                dimensions: (
                    MacroDecimal::Expression(format!("$1x{:.6}", length)),
                    MacroDecimal::Variable(2),
                ),
                center: (
                    MacroDecimal::Expression(format!("$1x({:.6}x$3-{:.6}x$4)", mid_x, mid_y)),
                    MacroDecimal::Expression(format!("$1x({:.6}x$4+{:.6}x$3)", mid_x, mid_y)),
                ),
                // here, we're using the same precision for rotation as the points themselves
                angle: MacroDecimal::Expression(format!("{:.6}+$5", rotation)),
            });
            content.push(circle_item);
        }

        vec![Command::ExtendedCode(ExtendedCode::ApertureMacro(ApertureMacro {
            name,
            content,
        }))]
    }

    #[cfg(test)]
    mod tests {
        use gerber_types::{
            ApertureMacro, CenterLinePrimitive, CirclePrimitive, Command, ExtendedCode, MacroBoolean, MacroContent,
            MacroDecimal,
        };

        use crate::testing::dump_gerber_source;
        use crate::testing::geometry::{
            calculate_alternating_points, compute_edge_rotations, extract_edges_and_midpoints,
        };

        /// A 5-point star, in this case, a pentagram, can be calculated from 10 points along the circumference of 2 circles.
        #[test]
        fn gen_star5_outline_macro_with_midpoints() {
            // given
            let outer_diameter = 1.0;
            let inner_diameter = 0.368;
            let center_x = 0.0;
            let center_y = 0.0;

            println!("outer_diameter: {}, inner_diameter: {}", outer_diameter, inner_diameter);
            println!("center_x: {}, center_y: {}", center_x, center_y);

            let mut content = vec![
                MacroContent::Comment("$1 = outer diameter (scale)".to_string()),
                MacroContent::Comment("$2 = line width".to_string()),
                MacroContent::Comment("$3 = cos(rotation in degrees)".to_string()),
                MacroContent::Comment("$4 = sin(rotation in degrees)".to_string()),
                MacroContent::Comment("$5 = rotation in degrees".to_string()),
                MacroContent::Comment("gerber expressions do not support trigonometry functions like sin/cos, so they have to be pre-computed and supplied".to_string()),
            ];

            content.push(MacroContent::Comment("end-points".to_string()));
            let points = calculate_alternating_points(outer_diameter, inner_diameter, center_x, center_y, 10);
            println!("points: {:?}", points);

            let (edges, midpoints) = extract_edges_and_midpoints(&points);
            println!("edges: {:?}", edges);
            println!("midpoints: {:?}", midpoints);

            let edge_rotations = compute_edge_rotations(&edges);
            println!("edge_rotations: {:?}", edge_rotations);

            for points in points.chunks_exact(2) {
                let build_end_point = |(x, y): &(f64, f64)| {
                    let circle_item = MacroContent::Circle(CirclePrimitive {
                        exposure: MacroBoolean::Value(true),
                        diameter: MacroDecimal::Variable(2),
                        center: (
                            MacroDecimal::Expression(format!("$1x{:.6}", x)),
                            MacroDecimal::Expression(format!("$1x{:.6}", y)),
                        ),
                        angle: Some(MacroDecimal::Variable(5)),
                    });
                    circle_item
                };

                for point in points {
                    content.push(build_end_point(point));
                }
            }

            content.push(MacroContent::Comment("mid-points".to_string()));
            for (mid_x, mid_y) in midpoints.iter() {
                let circle_item = MacroContent::Circle(CirclePrimitive {
                    exposure: MacroBoolean::Value(true),
                    diameter: MacroDecimal::Expression("$2x1.5".to_string()),
                    center: (
                        MacroDecimal::Expression(format!("$1x{:.6}", mid_x)),
                        MacroDecimal::Expression(format!("$1x{:.6}", mid_y)),
                    ),
                    angle: Some(MacroDecimal::Variable(5)),
                });
                content.push(circle_item);
            }

            content.push(MacroContent::Comment("center-lines".to_string()));
            for (index, ((((x1, y1), (x2, y2)), (mid_x, mid_y)), rotation)) in edges
                .iter()
                .zip(midpoints.iter())
                .zip(edge_rotations.iter())
                .enumerate()
            {
                let dx = x2 - x1;
                let dy = y2 - y1;
                let length = (dx * dx + dy * dy).sqrt();

                let angle_rad = dy.atan2(dx);
                let angle_deg = angle_rad.to_degrees();

                println!(
                    "line {}: dx: {}, dy: {}, length: {}, angle (old): {}, rotation (new): {}",
                    index, dx, dy, length, angle_deg, rotation
                );

                let circle_item = MacroContent::CenterLine(CenterLinePrimitive {
                    exposure: MacroBoolean::Value(true),
                    dimensions: (
                        MacroDecimal::Expression(format!("$1x{:.6}", length)),
                        MacroDecimal::Variable(2),
                    ),
                    center: (
                        MacroDecimal::Expression(format!("$1x({:.6}x$3-{:.6}x$4)", mid_x, mid_y)),
                        MacroDecimal::Expression(format!("$1x({:.6}x$4+{:.6}x$3)", mid_x, mid_y)),
                    ),
                    // here, we're using the same precision for rotation as the points themselves
                    angle: MacroDecimal::Expression(format!("{:.6}+$5", rotation)),
                });
                content.push(circle_item);
            }

            let expected_commands = vec![Command::ExtendedCode(ExtendedCode::ApertureMacro(ApertureMacro {
                name: "STAR5OUTLINEMP".to_string(),
                content,
            }))];

            // when
            let commands = super::generate_star_outline_macro_with_midpoints(
                outer_diameter,
                inner_diameter,
                "STAR5OUTLINEMP".to_string(),
            );

            // then
            dump_gerber_source(&commands);
            assert_eq!(commands, expected_commands);
        }

        /// A diamond can calculated from 4 points along the circumference of 2 circles.
        /// just simple asserts for this one; the implementation is the same as for the `gen_star5_outline_macro_with_midpoints` test
        #[test]
        pub fn gen_diamond_outline_macro() {
            // given
            let outer_diameter = 1.0;
            let inner_diameter = 0.75;

            // when
            let commands = super::generate_diamond_outline_macro_with_midpoints(
                outer_diameter,
                inner_diameter,
                "DIAMOND1OUTLINEMP".to_string(),
            );

            // then
            dump_gerber_source(&commands);
            assert!(!commands.is_empty());
        }

        /// A 4-point star can calculated from 8 points along the circumference of 2 circles.
        /// just simple asserts for this one; the implementation is the same as for the `gen_star5_outline_macro_with_midpoints` test
        #[test]
        pub fn gen_star4_outline_macro() {
            // given
            let outer_diameter = 1.0;
            let inner_diameter = 0.5;

            // when
            let commands = super::generate_alternating_shape_outline_macro(
                outer_diameter,
                inner_diameter,
                8,
                true,
                "STAR4OUTLINEMP".to_string(),
            );

            // then
            dump_gerber_source(&commands);
            assert!(!commands.is_empty());
        }

        /// A 4-point star can calculated from 8 points along the circumference of 2 circles.
        /// just simple asserts for this one; the implementation is the same as for the `gen_star5_outline_macro_with_midpoints` test
        #[test]
        pub fn gen_star6_outline_macro() {
            // given
            let outer_diameter = 1.0;
            let inner_diameter = 0.368;

            // when
            let commands = super::generate_alternating_shape_outline_macro(
                outer_diameter,
                inner_diameter,
                12,
                true,
                "STAR6OUTLINEMP".to_string(),
            );

            // then
            dump_gerber_source(&commands);
            assert!(!commands.is_empty());
        }
    }
}
