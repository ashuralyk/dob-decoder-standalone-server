use lazy_regex::regex;
use serde_json::Value;

pub static ARRAY_REG: lazy_regex::Lazy<regex::Regex> =
    lazy_regex::lazy_regex!(r"\%(.*?)\):(\[.*?\])");
pub static ARRAY_INDEX_REG: lazy_regex::Lazy<regex::Regex> = lazy_regex::lazy_regex!(r"(\d+)<_>$");
pub static GLOBAL_TEMPLATE_REG: lazy_regex::Lazy<regex::Regex> =
    lazy_regex::lazy_regex!(r"^prev<(.*?)>");
pub static TEMPLATE_REG: lazy_regex::Lazy<regex::Regex> = lazy_regex::lazy_regex!(r"^(.*?)<(.*?)>");

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
