use lazy_regex::regex;
use lyon::path::builder::NoAttributes;
use lyon::path::{BuilderImpl, Path};
use ttf_parser::{Face, OutlineBuilder};

use crate::svg::puretext::font::{TURRETROAD_400_TTF, TURRETROAD_700_TTF};

pub fn pathify_svg_texts(svg_content: &str) -> String {
    // Use regex to find and replace text elements
    let text_regex = regex!(r#"<text[^>]*>([^<]*)</text>"#);
    let mut result = svg_content.to_string();

    // Find all text elements and replace them with paths
    for cap in text_regex.captures_iter(svg_content) {
        let full_match = cap.get(0).unwrap().as_str();
        let text_content = cap.get(1).unwrap().as_str();

        // Extract attributes from the text element
        let x = extract_attribute(full_match, "x").unwrap_or(0.0);
        let y = extract_attribute(full_match, "y").unwrap_or(0.0);
        let font_size = extract_attribute(full_match, "font-size").unwrap_or(16.0);
        let font_weight =
            extract_string_attribute(full_match, "font-weight").unwrap_or("400".to_string());
        let fill_color =
            extract_string_attribute(full_match, "fill").unwrap_or("#000000".to_string());

        // Convert text to paths
        let paths = text_to_paths(text_content, x, y, font_size, &font_weight);

        // Replace the text element with path elements
        let mut path_elements = String::new();
        for path_data in paths {
            path_elements.push_str(&format!(
                r#"<path d="{}" fill="{}"/>"#,
                path_data, fill_color
            ));
        }

        result = result.replace(full_match, &path_elements);
    }

    result
}

fn extract_attribute(text: &str, attr_name: &str) -> Option<f32> {
    // Look for attribute="value" pattern
    let attr_pattern = format!(r#"{}=""#, attr_name);
    if let Some(start) = text.find(&attr_pattern) {
        // Find the start of the value (after the opening quote)
        let value_start = start + attr_pattern.len();
        if let Some(end) = text[value_start..].find('"') {
            let value = &text[value_start..value_start + end];
            return value.parse::<f32>().ok();
        }
    }
    None
}

fn extract_string_attribute(text: &str, attr_name: &str) -> Option<String> {
    // Look for attribute="value" pattern
    let attr_pattern = format!(r#"{}=""#, attr_name);
    if let Some(start) = text.find(&attr_pattern) {
        // Find the start of the value (after the opening quote)
        let value_start = start + attr_pattern.len();
        if let Some(end) = text[value_start..].find('"') {
            let value = &text[value_start..value_start + end];
            return Some(value.to_string());
        }
    }
    None
}

fn text_to_paths(text: &str, x: f32, y: f32, font_size: f32, font_weight: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut cursor_x = x;

    let font: &Face = if font_weight == "700" {
        &TURRETROAD_700_TTF
    } else {
        &TURRETROAD_400_TTF
    };

    // Scale factor (TTF font units to SVG coordinates)
    let scale = font_size / font.units_per_em() as f32;

    // Get font ascent for Y coordinate adjustment
    let ascent = font.ascender() as f32 * scale;

    for char in text.chars() {
        if let Some(glyph_id) = font.glyph_index(char) {
            // Create independent path builder for each character
            let mut builder = Path::builder();

            // Build glyph outline
            let mut outline_builder = LyonOutlineBuilder {
                path_builder: &mut builder,
                x_offset: cursor_x,
                y_offset: y + ascent, // Use font ascent for proper baseline
                scale,
            };

            // Get glyph outline
            if font.outline_glyph(glyph_id, &mut outline_builder).is_some() {
                // Complete path building
                let path = builder.build();

                // Convert to SVG path data
                let path_data = path_to_svg_path(&path);
                if !path_data.is_empty() {
                    paths.push(path_data);
                }
            }

            // Update cursor_x (glyph spacing)
            let advance = font.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * scale;
            cursor_x += advance;
        }
    }

    paths
}

// Lyon path builder (for ttf-parser)
struct LyonOutlineBuilder<'a> {
    path_builder: &'a mut NoAttributes<BuilderImpl>,
    x_offset: f32,
    y_offset: f32,
    scale: f32,
}

impl OutlineBuilder for LyonOutlineBuilder<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path_builder.begin(lyon::math::point(
            x * self.scale + self.x_offset,
            -y * self.scale + self.y_offset, // Flip Y axis to correct orientation
        ));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path_builder.line_to(lyon::math::point(
            x * self.scale + self.x_offset,
            -y * self.scale + self.y_offset,
        ));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path_builder.quadratic_bezier_to(
            lyon::math::point(
                x1 * self.scale + self.x_offset,
                -y1 * self.scale + self.y_offset,
            ),
            lyon::math::point(
                x * self.scale + self.x_offset,
                -y * self.scale + self.y_offset,
            ),
        );
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path_builder.cubic_bezier_to(
            lyon::math::point(
                x1 * self.scale + self.x_offset,
                -y1 * self.scale + self.y_offset,
            ),
            lyon::math::point(
                x2 * self.scale + self.x_offset,
                -y2 * self.scale + self.y_offset,
            ),
            lyon::math::point(
                x * self.scale + self.x_offset,
                -y * self.scale + self.y_offset,
            ),
        );
    }

    fn close(&mut self) {
        self.path_builder.close();
    }
}

// Convert Lyon path to SVG path data
fn path_to_svg_path(path: &Path) -> String {
    let mut d = String::new();
    for event in path.iter() {
        match event {
            lyon::path::Event::Begin { at } => {
                d.push_str(&format!("M{} {}", at.x, at.y));
            }
            lyon::path::Event::Line { to, .. } => {
                d.push_str(&format!("L{} {}", to.x, to.y));
            }
            lyon::path::Event::Quadratic { ctrl, to, .. } => {
                d.push_str(&format!("Q{} {} {} {}", ctrl.x, ctrl.y, to.x, to.y));
            }
            lyon::path::Event::Cubic {
                ctrl1, ctrl2, to, ..
            } => {
                d.push_str(&format!(
                    "C{} {} {} {} {} {}",
                    ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y
                ));
            }
            lyon::path::Event::End { close, .. } => {
                if close {
                    d.push('Z');
                }
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pathify_svg_texts() {
        let svg_input = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <text x="20" y="20" font-weight="400" font-size="16" fill="black">Hello</text>
            <text x="40" y="40" font-weight="700" font-size="16" fill="black">World</text>
            <circle cx="50" cy="50" r="10" fill="red"/>
        </svg>"#;

        let svg_output = pathify_svg_texts(svg_input);

        // Should contain path elements instead of text elements
        assert!(svg_output.contains("<path"));
        assert!(!svg_output.contains("<text"));
        // Should preserve non-text elements
        assert!(svg_output.contains("<circle"));

        println!("svg_output: {}", svg_output);
    }
}
