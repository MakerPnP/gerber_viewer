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

    /// generate star points, starting with the point at the top of the star, alternating between outer and inner radius
    pub fn calculate_star_points(
        outer_radius: f64,
        inner_radius: f64,
        center_x: f64,
        center_y: f64,
    ) -> Vec<(f64, f64)> {
        let mut points = Vec::new();
        let angle_step = (2.0 * PI) / 10.0; // 36 degrees in radians

        for i in 0..10 {
            let radius = if i % 2 == 0 { outer_radius } else { inner_radius };
            let angle = angle_step * i as f64 - PI / 2.0;

            let x = center_x + radius * angle.cos();
            let y = center_y - radius * angle.sin();

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

    #[cfg(test)]
    mod tests {
        use super::*;

        const STAR5_POINTS: [(f64, f64); 10] = [
            (6.123233995736766e-17, 1.0),
            (0.29389262614623657, 0.4045084971874737),
            (0.9510565162951535, 0.3090169943749474),
            (0.47552825814757677, -0.1545084971874737),
            (0.5877852522924731, -0.8090169943749475),
            (3.061616997868383e-17, -0.5),
            (-0.587785252292473, -0.8090169943749475),
            (-0.47552825814757677, -0.15450849718747375),
            (-0.9510565162951536, 0.3090169943749473),
            (-0.2938926261462366, 0.40450849718747367),
        ];

        #[test]
        fn star_points() {
            // given
            let outer_diameter = 1.0;
            let inner_diameter = 0.5;

            let expected_points = Vec::from(STAR5_POINTS);
            // when
            let points = calculate_star_points(outer_diameter, inner_diameter, 0.0, 0.0);

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
                ((6.123233995736766e-17, 1.0), (0.29389262614623657, 0.4045084971874737)),
                (
                    (0.29389262614623657, 0.4045084971874737),
                    (0.9510565162951535, 0.3090169943749474),
                ),
                (
                    (0.9510565162951535, 0.3090169943749474),
                    (0.47552825814757677, -0.1545084971874737),
                ),
                (
                    (0.47552825814757677, -0.1545084971874737),
                    (0.5877852522924731, -0.8090169943749475),
                ),
                ((0.5877852522924731, -0.8090169943749475), (3.061616997868383e-17, -0.5)),
                ((3.061616997868383e-17, -0.5), (-0.587785252292473, -0.8090169943749475)),
                (
                    (-0.587785252292473, -0.8090169943749475),
                    (-0.47552825814757677, -0.15450849718747375),
                ),
                (
                    (-0.47552825814757677, -0.15450849718747375),
                    (-0.9510565162951536, 0.3090169943749473),
                ),
                (
                    (-0.9510565162951536, 0.3090169943749473),
                    (-0.2938926261462366, 0.40450849718747367),
                ),
                ((-0.2938926261462366, 0.40450849718747367), (6.123233995736766e-17, 1.0)),
            ];

            let expected_midpoints = vec![
                (0.1469463130731183, 0.7022542485937369),
                (0.622474571220695, 0.3567627457812106),
                (0.7132923872213651, 0.07725424859373685),
                (0.531656755220025, -0.4817627457812106),
                (0.29389262614623657, -0.6545084971874737),
                (-0.2938926261462365, -0.6545084971874737),
                (-0.5316567552200249, -0.4817627457812106),
                (-0.7132923872213652, 0.07725424859373677),
                (-0.6224745712206952, 0.3567627457812105),
                (-0.14694631307311828, 0.7022542485937369),
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

    use crate::testing::geometry::{calculate_star_points, extract_edges_and_midpoints};

    /// used to generate code for demo gerber files
    #[allow(dead_code)]
    fn generate_star_outline_macro(outer_diameter: f64, inner_diameter: f64) -> Vec<Command> {
        generate_star_outline_macro_inner(outer_diameter, inner_diameter, false)
    }

    /// used to generate code for demo gerber files
    ///
    /// midpoints are handy for debugging gerber primitive calculations and gerber rendering
    #[allow(dead_code)]
    fn generate_star_outline_macro_with_midpoints(outer_diameter: f64, inner_diameter: f64) -> Vec<Command> {
        generate_star_outline_macro_inner(outer_diameter, inner_diameter, true)
    }

    fn generate_star_outline_macro_inner(
        outer_diameter: f64,
        inner_diameter: f64,
        with_midpoints: bool,
    ) -> Vec<Command> {
        let mut content = vec![
            MacroContent::Comment("$1 = outer diameter (scale)".to_string()),
            MacroContent::Comment("$2 = line width".to_string()),
        ];

        content.push(MacroContent::Comment("end-points".to_string()));
        let star_points = calculate_star_points(outer_diameter, inner_diameter, 0.0, 0.0);

        let (edges, midpoints) = extract_edges_and_midpoints(&star_points);

        for points in star_points.chunks_exact(2) {
            let build_end_point = |(x, y): &(f64, f64)| {
                let (formatted_x, formatted_y) = (format!("{:.4}", x), format!("{:.4}", y));

                let circle_item = MacroContent::Circle(CirclePrimitive {
                    exposure: MacroBoolean::Value(true),
                    diameter: MacroDecimal::Variable(2),
                    center: (
                        MacroDecimal::Expression(format!("$1x{}", formatted_x)),
                        MacroDecimal::Expression(format!("$1x{}", formatted_y)),
                    ),
                    angle: None,
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
                let (formatted_x, formatted_y) = (format!("{:.4}", mid_x), format!("{:.4}", mid_y));

                let circle_item = MacroContent::Circle(CirclePrimitive {
                    exposure: MacroBoolean::Value(true),
                    diameter: MacroDecimal::Expression("$2x1.5".to_string()),
                    center: (
                        MacroDecimal::Expression(format!("$1x{}", formatted_x)),
                        MacroDecimal::Expression(format!("$1x{}", formatted_y)),
                    ),
                    angle: None,
                });
                content.push(circle_item);
            }
        }

        content.push(MacroContent::Comment("center-lines".to_string()));
        for (((x1, y1), (x2, y2)), (mid_x, mid_y)) in edges.iter().zip(midpoints.iter()) {
            let dx = x2 - x1;
            let dy = y2 - y1;
            let length = (dx * dx + dy * dy).sqrt();

            let angle_rad = dy.atan2(dx);
            let angle_deg = angle_rad.to_degrees();

            let (formatted_x, formatted_y) = (format!("{:.4}", mid_x), format!("{:.4}", mid_y));

            let circle_item = MacroContent::CenterLine(CenterLinePrimitive {
                exposure: MacroBoolean::Value(true),
                dimensions: (
                    MacroDecimal::Expression(format!("$1x{}", length)),
                    MacroDecimal::Variable(2),
                ),
                center: (
                    MacroDecimal::Expression(format!("$1x{}", formatted_x)),
                    MacroDecimal::Expression(format!("$1x{}", formatted_y)),
                ),
                angle: MacroDecimal::Value(angle_deg),
            });
            content.push(circle_item);
        }

        vec![Command::ExtendedCode(ExtendedCode::ApertureMacro(ApertureMacro {
            name: "STAR5OUTLINE".to_string(),
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
        use crate::testing::geometry::{calculate_star_points, extract_edges_and_midpoints};

        #[test]
        fn gen_star_outline_macro_with_midpoints() {
            // given
            let outer_diameter = 1.0;
            let inner_diameter = 0.368;

            let mut content = vec![
                MacroContent::Comment("$1 = outer diameter (scale)".to_string()),
                MacroContent::Comment("$2 = line width".to_string()),
            ];

            content.push(MacroContent::Comment("end-points".to_string()));
            let star_points = calculate_star_points(outer_diameter, inner_diameter, 0.0, 0.0);

            let (edges, midpoints) = extract_edges_and_midpoints(&star_points);

            for points in star_points.chunks_exact(2) {
                let build_end_point = |(x, y): &(f64, f64)| {
                    let (formatted_x, formatted_y) = (format!("{:.4}", x), format!("{:.4}", y));

                    let circle_item = MacroContent::Circle(CirclePrimitive {
                        exposure: MacroBoolean::Value(true),
                        diameter: MacroDecimal::Variable(2),
                        center: (
                            MacroDecimal::Expression(format!("$1x{}", formatted_x)),
                            MacroDecimal::Expression(format!("$1x{}", formatted_y)),
                        ),
                        angle: None,
                    });
                    circle_item
                };

                for point in points {
                    content.push(build_end_point(point));
                }
            }

            content.push(MacroContent::Comment("mid-points".to_string()));
            for (mid_x, mid_y) in midpoints.iter() {
                let (formatted_x, formatted_y) = (format!("{:.4}", mid_x), format!("{:.4}", mid_y));

                let circle_item = MacroContent::Circle(CirclePrimitive {
                    exposure: MacroBoolean::Value(true),
                    diameter: MacroDecimal::Expression("$2x1.5".to_string()),
                    center: (
                        MacroDecimal::Expression(format!("$1x{}", formatted_x)),
                        MacroDecimal::Expression(format!("$1x{}", formatted_y)),
                    ),
                    angle: None,
                });
                content.push(circle_item);
            }

            content.push(MacroContent::Comment("center-lines".to_string()));
            for (((x1, y1), (x2, y2)), (mid_x, mid_y)) in edges.iter().zip(midpoints.iter()) {
                let dx = x2 - x1;
                let dy = y2 - y1;
                let length = (dx * dx + dy * dy).sqrt();

                let angle_rad = dy.atan2(dx);
                let angle_deg = angle_rad.to_degrees();

                let (formatted_x, formatted_y) = (format!("{:.4}", mid_x), format!("{:.4}", mid_y));

                let circle_item = MacroContent::CenterLine(CenterLinePrimitive {
                    exposure: MacroBoolean::Value(true),
                    dimensions: (
                        MacroDecimal::Expression(format!("$1x{}", length)),
                        MacroDecimal::Variable(2),
                    ),
                    center: (
                        MacroDecimal::Expression(format!("$1x{}", formatted_x)),
                        MacroDecimal::Expression(format!("$1x{}", formatted_y)),
                    ),
                    angle: MacroDecimal::Value(angle_deg),
                });
                content.push(circle_item);
            }

            let expected_commands = vec![Command::ExtendedCode(ExtendedCode::ApertureMacro(ApertureMacro {
                name: "STAR5OUTLINE".to_string(),
                content,
            }))];

            // when
            let commands = super::generate_star_outline_macro_with_midpoints(outer_diameter, inner_diameter);

            // then
            dump_gerber_source(&commands);
            assert_eq!(commands, expected_commands);
        }
    }
}
