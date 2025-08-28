use crate::svg::puretext::{constants::Key, SimpleDOBOutput, TraitExt as _};

pub fn get_background_color_by_traits(traits: &[SimpleDOBOutput]) -> Option<&SimpleDOBOutput> {
    traits
        .iter()
        .find(|trait_| trait_.name == Key::BgColor.to_string())
}

pub struct BackgroundColorOptions {
    pub default_color: Option<String>,
}

impl Default for BackgroundColorOptions {
    fn default() -> Self {
        Self {
            default_color: Some("#000".to_string()),
        }
    }
}

pub fn background_color_parser(
    traits: &[SimpleDOBOutput],
    options: Option<BackgroundColorOptions>,
) -> String {
    let bg_color_trait = get_background_color_by_traits(traits);

    if let Some(trait_) = bg_color_trait {
        if let Some(value) = trait_.get_string_value() {
            if value.starts_with("#(") && value.ends_with(')') {
                return value.replace("#(", "linear-gradient(");
            }
            return value.to_string();
        }
    }

    options
        .and_then(|opt| opt.default_color)
        .unwrap_or_else(|| "#000".to_string())
}
