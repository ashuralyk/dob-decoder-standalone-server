use serde_json::Value;

pub mod constants;
pub mod font;
pub mod parsers;
pub mod render;

#[derive(Clone, Debug)]
pub struct SimpleDOBOutput {
    pub name: String,
    pub value: Value,
}

pub trait TraitExt {
    fn get_string_value(&self) -> Option<&str>;
    fn get_number_value(&self) -> Option<u64>;
    fn get_timestamp_value(&self) -> Option<u64>;
    fn get_svg_value(&self) -> Option<&str>;
}

impl TraitExt for SimpleDOBOutput {
    fn get_string_value(&self) -> Option<&str> {
        self.value.as_str()
    }

    fn get_number_value(&self) -> Option<u64> {
        self.value.as_u64()
    }

    fn get_timestamp_value(&self) -> Option<u64> {
        self.value.as_u64()
    }

    fn get_svg_value(&self) -> Option<&str> {
        self.value.as_str()
    }
}
