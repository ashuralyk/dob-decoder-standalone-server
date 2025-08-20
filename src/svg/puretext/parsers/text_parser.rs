use std::collections::HashMap;

use serde_json::Value;

use crate::svg::puretext::{
    constants::{key::Key, regex::*},
    parsers::{
        background_parser::{background_color_parser, BackgroundColorOptions},
        style_parser::{
            style_parser, ParsedStyle, ParsedStyleAlignment, ParsedStyleFormat, StyleParserOptions,
        },
    },
    SimpleDOBOutput, TraitExt as _,
};

pub const DEFAULT_TEMPLATE: &str = "%k: %v";

#[derive(Debug, Clone)]
pub struct TextParserOptions {
    pub default_template: Option<String>,
}

impl Default for TextParserOptions {
    fn default() -> Self {
        Self {
            default_template: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StyleCss {
    pub text_align: Option<String>,
    pub color: Option<String>,
    pub font_weight: Option<String>,
    pub font_style: Option<String>,
    pub text_decoration: Option<String>,
}

impl Default for StyleCss {
    fn default() -> Self {
        Self {
            text_align: None,
            color: None,
            font_weight: None,
            font_style: None,
            text_decoration: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextItem {
    pub name: String,
    pub value: Value,
    pub parsed_style: ParsedStyle,
    pub template: String,
    pub text: String,
    pub style: StyleCss,
}

#[derive(Debug, Clone)]
pub struct TextParserResult {
    pub items: Vec<TextItem>,
    pub bg_color: String,
}

pub fn render_text_params_parser(
    traits: &[SimpleDOBOutput],
    index_var_register: &HashMap<String, u32>,
    options: Option<TextParserOptions>,
) -> TextParserResult {
    let bg_color = background_color_parser(
        traits,
        Some(BackgroundColorOptions {
            default_color: Some("#000".to_string()),
        }),
    );

    let mut template = options
        .as_ref()
        .and_then(|opt| opt.default_template.clone())
        .unwrap_or_else(|| DEFAULT_TEMPLATE.to_string());

    let mut style = style_parser("", None);

    // Find global template trait
    let global_template_trait = traits
        .iter()
        .find(|trait_| GLOBAL_TEMPLATE_REG.is_match(&trait_.name));

    if let Some(global_trait) = global_template_trait {
        if let Some(value) = global_trait.get_string_value() {
            let mut processed_value = value.to_string();
            if !processed_value.starts_with('<') && !processed_value.ends_with('>') {
                processed_value = format!("<{}>", processed_value);
            }
            style = style_parser(&processed_value, None);
        }

        // Extract template from trait name
        if let Some(captures) = TEMPLATE_REG.captures(&global_trait.name) {
            if let Some(template_match) = captures.get(2) {
                template = template_match.as_str().to_string();
            }
        }
    }

    let items: Vec<TextItem> = traits
        .iter()
        .filter(|trait_| {
            !trait_.name.starts_with(&Key::Prev.to_string())
                && !index_var_register.contains_key(&trait_.name)
                && trait_.name != Key::Image.to_string()
        })
        .map(|trait_| {
            let mut current_template = template.clone();
            let mut parsed_style = style.clone();
            let mut processed_value = trait_.value.clone();
            let name = trait_.name.clone();

            // Parse value for layout and style
            if let Some(value_str) = trait_.get_string_value() {
                if let Some(captures) = TEMPLATE_REG.captures(&value_str) {
                    if let Some(value_match) = captures.get(1) {
                        processed_value = Value::String(value_match.as_str().to_string());
                    }
                    if let Some(style_match) = captures.get(2) {
                        let style_str = format!("<{}>", style_match.as_str());
                        parsed_style = style_parser(
                            &style_str,
                            Some(StyleParserOptions {
                                base_style: Some(parsed_style.clone()),
                            }),
                        );
                    }
                }
            }

            // Parse name for template
            let mut processed_name = name.clone();
            if let Some(captures) = TEMPLATE_REG.captures(&name) {
                if let Some(name_match) = captures.get(1) {
                    processed_name = name_match.as_str().to_string();
                }
                if let Some(template_match) = captures.get(2) {
                    current_template = template_match.as_str().to_string();
                }
            }

            // Generate text from template
            let text = current_template
                .replace("%k", &processed_name)
                .replace("%v", &parse_value_to_string(&processed_value))
                .replace("%%", "%");

            // Generate CSS style
            let mut style_css = StyleCss::default();

            match parsed_style.alignment {
                ParsedStyleAlignment::Left => {
                    style_css.text_align = Some("left".to_string());
                }
                ParsedStyleAlignment::Center => {
                    style_css.text_align = Some("center".to_string());
                }
                ParsedStyleAlignment::Right => {
                    style_css.text_align = Some("right".to_string());
                }
            }

            if !parsed_style.color.is_empty() {
                style_css.color = Some(parsed_style.color.clone());
            }

            for format in &parsed_style.format {
                match format {
                    ParsedStyleFormat::Bold => {
                        style_css.font_weight = Some("700".to_string());
                    }
                    ParsedStyleFormat::Italic => {
                        style_css.font_style = Some("italic".to_string());
                    }
                    ParsedStyleFormat::Underline => {
                        style_css.text_decoration = Some("underline".to_string());
                    }
                    ParsedStyleFormat::Strikethrough => {
                        style_css.text_decoration = Some("line-through".to_string());
                    }
                }
            }

            TextItem {
                name: processed_name,
                value: processed_value,
                parsed_style,
                template: current_template,
                text,
                style: style_css,
            }
        })
        .collect();

    TextParserResult { items, bg_color }
}
