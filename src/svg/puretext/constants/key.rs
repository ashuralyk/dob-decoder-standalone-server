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
