use lazy_regex::{lazy_regex, regex, Lazy};
use serde_json::Value;

pub static ARRAY_REG: Lazy<regex::Regex> = lazy_regex!(r"\%(.*?)\):(\[.*?\])");
pub static ARRAY_INDEX_REG: Lazy<regex::Regex> = lazy_regex!(r"(\d+)<_>$");
pub static GLOBAL_TEMPLATE_REG: Lazy<regex::Regex> = lazy_regex!(r"^prev<(.*?)>");
pub static TEMPLATE_REG: Lazy<regex::Regex> = lazy_regex!(r"^(.*?)<(.*?)>");

pub fn parse_string_to_array(s: &str) -> Vec<String> {
    // This regex matches anything inside single quotes: '...'
    let re = regex::Regex::new(r"'([^']*)'").unwrap();
    re.captures_iter(s)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

pub fn parse_value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    BgColor,
    Prev,
    Image,
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::BgColor => write!(f, "prev.bgcolor"),
            Key::Prev => write!(f, "prev"),
            Key::Image => write!(f, "IMAGE"),
        }
    }
}

impl std::str::FromStr for Key {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "prev.bgcolor" => Ok(Key::BgColor),
            "prev" => Ok(Key::Prev),
            "IMAGE" => Ok(Key::Image),
            _ => Err(format!("Unknown key: {}", s)),
        }
    }
}
