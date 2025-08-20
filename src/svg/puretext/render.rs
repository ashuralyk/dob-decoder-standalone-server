use crate::svg::puretext::parsers::style_parser::ParsedStyleAlignment;
use crate::svg::puretext::parsers::text_parser::{TextItem, TextParserResult};

#[derive(Debug, Clone)]
pub struct RenderProps {
    pub items: Vec<TextItem>,
    pub bg_color: String,
}

impl From<TextParserResult> for RenderProps {
    fn from(result: TextParserResult) -> Self {
        Self {
            items: result.items,
            bg_color: result.bg_color,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderElement {
    pub key: String,
    pub element_type: String,
    pub props: ElementProps,
}

#[derive(Debug, Clone)]
pub struct ElementProps {
    pub children: Vec<String>,
    pub style: StyleProps,
}

#[derive(Debug, Clone)]
pub struct StyleProps {
    pub display: Option<String>,
    pub justify_content: Option<String>,
    pub flex_wrap: Option<String>,
    pub width: Option<String>,
    pub margin: Option<String>,
    pub height: Option<String>,
    pub text_align: Option<String>,
    pub color: Option<String>,
    pub font_weight: Option<String>,
    pub font_style: Option<String>,
    pub text_decoration: Option<String>,
}

impl Default for StyleProps {
    fn default() -> Self {
        Self {
            display: None,
            justify_content: None,
            flex_wrap: None,
            width: None,
            margin: None,
            height: None,
            text_align: None,
            color: None,
            font_weight: None,
            font_style: None,
            text_decoration: None,
        }
    }
}

pub fn render_text_svg(props: RenderProps) -> String {
    // Convert text items to render elements
    let children = convert_items_to_elements(&props.items);

    // Generate SVG
    generate_svg(&children, &props.bg_color)
}

fn convert_items_to_elements(items: &[TextItem]) -> Vec<RenderElement> {
    let mut elements = Vec::new();

    for item in items {
        let justify_content = match item.parsed_style.alignment {
            ParsedStyleAlignment::Left => "flex-start",
            ParsedStyleAlignment::Center => "center",
            ParsedStyleAlignment::Right => "flex-end",
        };

        let mut style = StyleProps::default();
        style.display = Some("flex".to_string());
        style.justify_content = Some(justify_content.to_string());
        style.flex_wrap = Some("wrap".to_string());
        style.width = Some("100%".to_string());
        style.margin = Some("0".to_string());

        // Apply text styling
        if let Some(color) = &item.style.color {
            style.color = Some(color.clone());
        }
        if let Some(font_weight) = &item.style.font_weight {
            style.font_weight = Some(font_weight.clone());
        }
        if let Some(font_style) = &item.style.font_style {
            style.font_style = Some(font_style.clone());
        }
        if let Some(text_decoration) = &item.style.text_decoration {
            style.text_decoration = Some(text_decoration.clone());
        }

        let element = RenderElement {
            key: item.name.clone(),
            element_type: "p".to_string(),
            props: ElementProps {
                children: vec![item.text.clone()],
                style: style.clone(),
            },
        };

        // Handle break line logic
        if item.parsed_style.break_line == 0 {
            // No line break - just add the element
            elements.push(element);
        } else {
            // Add the main element
            elements.push(element);

            // Add additional line breaks
            for i in 0..item.parsed_style.break_line {
                let break_element = RenderElement {
                    key: format!("{}-break-{}", item.name, i),
                    element_type: "p".to_string(),
                    props: ElementProps {
                        children: vec![],
                        style: StyleProps {
                            height: Some("36px".to_string()),
                            margin: Some("0".to_string()),
                            ..Default::default()
                        },
                    },
                };
                elements.push(break_element);
            }
        }
    }

    elements
}

fn generate_svg(elements: &[RenderElement], bg_color: &str) -> String {
    let width = 500;
    let padding_x = 20;
    let padding_y = 30;
    let line_height = 27;
    let font_size = 36;

    let mut svg_content = String::new();
    let mut current_y = padding_y + line_height; // Start after top padding
    let mut used_font_weights = std::collections::HashSet::new();

    // Calculate dynamic height based on content with minimum of 500
    let calculated_height = elements.len() * line_height + padding_y;
    let height = std::cmp::max(calculated_height as u32, 500);

    // First pass: collect used font weights
    for element in elements {
        if element.element_type == "p" {
            let font_weight_to_use = if let Some(font_weight) = &element.props.style.font_weight {
                if font_weight == "bold" {
                    "700"
                } else {
                    "400"
                }
            } else {
                "400"
            };
            used_font_weights.insert(font_weight_to_use.to_string());
        }
    }

    // Second pass: generate SVG content
    for element in elements {
        if element.element_type == "p" {
            if !element.props.children.is_empty() {
                let text = &element.props.children[0];
                if !text.is_empty() {
                    let text_anchor = match element.props.style.justify_content.as_deref() {
                        Some("center") => "middle",
                        Some("flex-end") => "end",
                        _ => "start",
                    };

                    let color = element.props.style.color.as_deref().unwrap_or("#ffffff");

                    // Determine font weight for Turret Road font family
                    let font_weight_to_use =
                        if let Some(font_weight) = &element.props.style.font_weight {
                            font_weight
                        } else {
                            "400"
                        };

                    let text_element = format!(
                        r#"<text x="{}" y="{}" font-family="Turret Road" font-weight="{}" font-size="{}" fill="{}" text-anchor="{}">{}</text>"#,
                        padding_x,
                        current_y,
                        font_weight_to_use,
                        font_size,
                        color,
                        text_anchor,
                        escape_xml(text)
                    );

                    svg_content.push_str(&text_element);
                    svg_content.push('\n');
                }
            }
            current_y += line_height;
        }
    }

    // Create font definitions with only used weights
    let font_defs = create_font_definitions(&used_font_weights);

    // Create gradient definition if bg_color is a gradient
    let (background_rect, gradient_defs) = if bg_color.starts_with("linear-gradient") {
        create_gradient_background(bg_color, width, height)
    } else {
        // Use solid color
        let rect = format!(
            r#"<rect width="{}" height="{}" fill="{}"/>"#,
            width, height, bg_color
        );
        (rect, String::new())
    };

    // Create the complete SVG
    format!(
        r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">{}{}{}{}</svg>"#,
        width, height, font_defs, gradient_defs, background_rect, svg_content
    )
}

fn create_gradient_background(gradient_css: &str, width: u32, height: u32) -> (String, String) {
    // Parse CSS linear-gradient format: linear-gradient(70deg, blue, pink, #f00)
    let gradient_id = "background-gradient";

    // Extract angle and colors from the gradient string
    let (angle, colors) = parse_gradient_css(gradient_css);

    // Calculate gradient coordinates based on angle
    let (x1, y1, x2, y2) = calculate_gradient_coordinates(angle);

    // Create gradient stops
    let mut stops = String::new();
    let num_colors = colors.len();
    for (i, color) in colors.iter().enumerate() {
        let offset = if num_colors == 1 {
            "0%".to_string()
        } else {
            format!("{}%", (i * 100) / (num_colors - 1))
        };
        stops.push_str(&format!(
            r#"<stop offset="{}" style="stop-color:{};stop-opacity:1" />"#,
            offset, color
        ));
    }

    // Create gradient definition
    let gradient_defs = format!(
        r#"<defs><linearGradient id="{}" x1="{}" y1="{}" x2="{}" y2="{}">{}</linearGradient></defs>"#,
        gradient_id, x1, y1, x2, y2, stops
    );

    // Create background rect with gradient fill
    let background_rect = format!(
        r#"<rect width="{}" height="{}" fill="url(#{}"/>"#,
        width, height, gradient_id
    );

    (background_rect, gradient_defs)
}

fn create_font_definitions(_used_font_weights: &std::collections::HashSet<String>) -> String {
    // Include the complete Google Fonts CSS for Turret Road font
    // This is the actual CSS fetched from: https://fonts.googleapis.com/css2?family=Turret+Road:wght@200;300;400;500;700;800&display=swap
    let mut font_defs = String::from(r#"<defs><style>"#);

    // Add the complete Google Fonts CSS
    font_defs.push_str(
        r#"
@font-face {
    font-family: 'Turret Road';
    font-style: normal;
    font-weight: 400;
    font-display: swap;
    src: url(https://fonts.gstatic.com/s/turretroad/v10/pxiAypMgpcBFjE84Zv-fE3tF.ttf) format('truetype');
}
@font-face {
    font-family: 'Turret Road';
    font-style: normal;
    font-weight: 700;
    font-display: swap;
    src: url(https://fonts.gstatic.com/s/turretroad/v10/pxidypMgpcBFjE84Zv-fE0P5FdeL.ttf) format('truetype');
}"#,
    );

    // Add the text element styling to enable the font
    font_defs.push_str(
        r#"
text {
    font-family: "Turret Road";
}"#,
    );

    font_defs.push_str(
        r#"
</style></defs>"#,
    );

    font_defs
}

fn calculate_gradient_coordinates(angle: f64) -> (String, String, String, String) {
    // For CSS linear-gradient, 0deg points to the right, 90deg points down
    // We need to calculate the gradient line that goes through the rectangle at the given angle

    // Normalize angle to 0-360 range
    let normalized_angle = angle % 360.0;

    // Convert angle to radians
    let angle_rad = normalized_angle.to_radians();

    // Calculate the direction vector
    let dx = angle_rad.sin();
    let dy = angle_rad.cos();

    // For a rectangle with width=100% and height=100%, we need to find
    // the intersection points of the gradient line with the rectangle boundaries
    // The gradient line passes through the center (50%, 50%) of the rectangle

    // Calculate the intersection points with the rectangle boundaries
    // We'll use parametric line equations to find where the line intersects the rectangle

    let center_x = 0.5; // 50%
    let center_y = 0.5; // 50%

    // Find the parameter t where the line intersects each boundary
    let mut t_values = Vec::new();

    // Intersection with left boundary (x = 0)
    if dx.abs() > 1e-10 {
        let t = -center_x / dx;
        let y = center_y + t * dy;
        if y >= 0.0 && y <= 1.0 {
            t_values.push((t, 0.0, y));
        }
    }

    // Intersection with right boundary (x = 1)
    if dx.abs() > 1e-10 {
        let t = (1.0 - center_x) / dx;
        let y = center_y + t * dy;
        if y >= 0.0 && y <= 1.0 {
            t_values.push((t, 1.0, y));
        }
    }

    // Intersection with top boundary (y = 0)
    if dy.abs() > 1e-10 {
        let t = -center_y / dy;
        let x = center_x + t * dx;
        if x >= 0.0 && x <= 1.0 {
            t_values.push((t, x, 0.0));
        }
    }

    // Intersection with bottom boundary (y = 1)
    if dy.abs() > 1e-10 {
        let t = (1.0 - center_y) / dy;
        let x = center_x + t * dx;
        if x >= 0.0 && x <= 1.0 {
            t_values.push((t, x, 1.0));
        }
    }

    // Sort by parameter t to get the two endpoints
    t_values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    if t_values.len() >= 2 {
        let (_, x1, y1) = t_values[0];
        let (_, x2, y2) = t_values[t_values.len() - 1];

        // Convert to percentage strings
        // For CSS linear-gradient, the gradient flows in the direction of the angle
        // So we want the start point to be where the gradient begins
        (
            format!("{:.1}%", x1 * 100.0),
            format!("{:.1}%", y1 * 100.0),
            format!("{:.1}%", x2 * 100.0),
            format!("{:.1}%", y2 * 100.0),
        )
    } else {
        // Fallback for edge cases
        (
            "0%".to_string(),
            "0%".to_string(),
            "100%".to_string(),
            "100%".to_string(),
        )
    }
}

fn parse_gradient_css(gradient_css: &str) -> (f64, Vec<String>) {
    // Parse: linear-gradient(70deg, blue, pink, #f00)
    let mut parts = gradient_css
        .trim_start_matches("linear-gradient(")
        .trim_end_matches(")")
        .split(',');

    // Extract angle
    let angle_part = parts.next().unwrap_or("0deg").trim();
    let angle = angle_part
        .trim_end_matches("deg")
        .parse::<f64>()
        .unwrap_or(0.0);

    // Extract colors
    let colors: Vec<String> = parts.map(|s| s.trim().to_string()).collect();

    (angle, colors)
}

fn escape_xml(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

// Legacy function for backward compatibility
pub fn render_text_params(render_output: Vec<crate::types::StandardDOBOutput>) -> String {
    use crate::svg::puretext::parsers::{text_parser, traits_parser};

    let traits_parser_result = traits_parser::dob_output_parser(&render_output);
    let text_parser_result = text_parser::render_text_params_parser(
        &traits_parser_result.traits,
        &traits_parser_result.index_var_register,
        None,
    );

    render_text_parser_result_to_svg(&text_parser_result)
}

pub fn render_text_parser_result_to_svg(text_parser_result: &TextParserResult) -> String {
    let props = RenderProps::from(text_parser_result.clone());
    render_text_svg(props)
}
