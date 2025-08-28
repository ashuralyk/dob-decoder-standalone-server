use serde_json::Value;
use std::collections::HashMap;

use crate::{
    svg::puretext::{
        constants::{parse_string_to_array, ARRAY_INDEX_REG, ARRAY_REG},
        SimpleDOBOutput, TraitExt,
    },
    types::{ParsedTrait, StandardDOBOutput},
};

#[derive(Clone, Debug)]
pub struct TraitsParserResult {
    pub traits: Vec<SimpleDOBOutput>,
    pub index_var_register: HashMap<String, u32>,
}

pub fn dob_output_parser(items: &[StandardDOBOutput]) -> TraitsParserResult {
    // Build index variable register
    let index_var_register = items
        .iter()
        .filter_map(|item| {
            let first_trait = item.traits.first()?;
            let string_value = first_trait.get_string_value()?;

            if let Some(captures) = ARRAY_INDEX_REG.captures(string_value) {
                if let Some(index_match) = captures.get(1) {
                    if let Ok(int_index) = index_match.as_str().parse::<u32>() {
                        return Some((item.name.clone(), int_index));
                    }
                }
            }
            None
        })
        .collect::<HashMap<String, u32>>();

    // Parse traits
    let traits = items
        .iter()
        .filter_map(|item| {
            let first_trait = item.traits.first()?;

            // Handle String traits
            if let Some(string_value) = first_trait.get_string_value() {
                let mut value = string_value.to_string();

                // Handle array indexing
                if let Some(captures) = ARRAY_REG.captures(&value) {
                    if let Some(var_name_match) = captures.get(1) {
                        if let Some(array_match) = captures.get(2) {
                            let var_name = var_name_match.as_str();
                            let array = parse_string_to_array(array_match.as_str());

                            if let Some(&index) = index_var_register.get(var_name) {
                                let array_index = (index as usize) % array.len();
                                value = array[array_index].clone();
                            }
                        }
                    }
                }

                return Some(SimpleDOBOutput {
                    name: item.name.clone(),
                    value: Value::String(value),
                });
            }

            // Handle Number traits
            if let Some(number_value) = first_trait.get_number_value() {
                return Some(SimpleDOBOutput {
                    name: item.name.clone(),
                    value: Value::Number(serde_json::Number::from(number_value)),
                });
            }

            // Handle Timestamp traits
            if let Some(timestamp_value) = first_trait.get_timestamp_value() {
                let mut timestamp = timestamp_value;

                // Convert 10-digit timestamp to milliseconds if needed
                if timestamp.to_string().len() == 10 {
                    timestamp *= 1000;
                }

                // Convert to ISO string format
                let timestamp_ms = timestamp as i64;
                let datetime = chrono::DateTime::from_timestamp_millis(timestamp_ms)?;
                let iso_string = datetime.to_rfc3339();

                return Some(SimpleDOBOutput {
                    name: item.name.clone(),
                    value: Value::String(iso_string),
                });
            }

            // Handle SVG traits
            if let Some(svg_value) = first_trait.get_svg_value() {
                let resolved_svg = resolve_svg_traits(svg_value);
                return Some(SimpleDOBOutput {
                    name: item.name.clone(),
                    value: Value::String(resolved_svg),
                });
            }

            None
        })
        .collect();

    TraitsParserResult {
        traits,
        index_var_register,
    }
}

fn resolve_svg_traits(svg: &str) -> String {
    // For now, just return the SVG as-is
    // This could be expanded to handle SVG trait resolution
    svg.to_string()
}

impl TraitExt for ParsedTrait {
    fn get_string_value(&self) -> Option<&str> {
        if self.type_ == "String" {
            self.value.as_str()
        } else {
            None
        }
    }

    fn get_number_value(&self) -> Option<u64> {
        if self.type_ == "Number" {
            self.value.as_u64()
        } else {
            None
        }
    }

    fn get_timestamp_value(&self) -> Option<u64> {
        if self.type_ == "Timestamp" {
            self.value.as_u64()
        } else {
            None
        }
    }

    fn get_svg_value(&self) -> Option<&str> {
        if self.type_ == "SVG" {
            self.value.as_str()
        } else {
            None
        }
    }
}
