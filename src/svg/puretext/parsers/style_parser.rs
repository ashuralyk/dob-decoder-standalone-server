use lazy_regex::regex;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedStyleFormat {
    Bold,
    Italic,
    Strikethrough,
    Underline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedStyleAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
pub struct ParsedStyle {
    pub color: String,
    pub format: Vec<ParsedStyleFormat>,
    pub alignment: ParsedStyleAlignment,
    pub break_line: u32,
}

impl Default for ParsedStyle {
    fn default() -> Self {
        Self {
            color: "#fff".to_string(),
            format: Vec::new(),
            alignment: ParsedStyleAlignment::Left,
            break_line: 1,
        }
    }
}

#[derive(Default)]
pub struct StyleParserOptions {
    pub base_style: Option<ParsedStyle>,
}

pub fn style_parser(input: &str, options: Option<StyleParserOptions>) -> ParsedStyle {
    let mut text = input.to_string();
    let mut result = options.and_then(|opt| opt.base_style).unwrap_or_default();

    // Remove angle brackets if present
    if text.starts_with('<') && text.ends_with('>') {
        text = text[1..text.len() - 1].to_string();
    }

    // Parse 6-digit hex color
    if let Some(captures) = regex!(r"#([0-9a-fA-F]{6})").captures(&text) {
        if let Some(color_match) = captures.get(1) {
            result.color = format!("#{}", color_match.as_str());
            text = regex!(r"#([0-9a-fA-F]{6})").replace(&text, "").to_string();
        }
    }

    // Parse 3-digit hex color
    if let Some(captures) = regex!(r"#([0-9a-fA-F]{3})").captures(&text) {
        if let Some(color_match) = captures.get(1) {
            result.color = format!("#{}", color_match.as_str());
            text = regex!(r"#([0-9a-fA-F]{3})").replace(&text, "").to_string();
        }
    }

    // Parse format specifiers (*bisu)
    if let Some(captures) = regex!(r"\*([bisu]+)").captures(&text) {
        if let Some(format_match) = captures.get(1) {
            let format_str = format_match.as_str();
            let mut formats = Vec::new();

            for ch in format_str.chars() {
                let format = match ch {
                    'b' => Some(ParsedStyleFormat::Bold),
                    'i' => Some(ParsedStyleFormat::Italic),
                    's' => Some(ParsedStyleFormat::Strikethrough),
                    'u' => Some(ParsedStyleFormat::Underline),
                    _ => None,
                };
                if let Some(fmt) = format {
                    formats.push(fmt);
                }
            }

            result.format = formats;
            text = regex!(r"\*([bisu]+)").replace(&text, "").to_string();
        }
    }

    // Parse alignment (@l, @c, @r)
    if let Some(captures) = regex!(r"@(l|c|r)").captures(&text) {
        if let Some(align_match) = captures.get(1) {
            result.alignment = match align_match.as_str() {
                "l" => ParsedStyleAlignment::Left,
                "c" => ParsedStyleAlignment::Center,
                "r" => ParsedStyleAlignment::Right,
                _ => ParsedStyleAlignment::Left,
            };
            text = regex!(r"@(l|c|r)").replace(&text, "").to_string();
        }
    }

    // Parse no line break (&)
    if regex!(r"&").is_match(&text) {
        result.break_line = 0;
        text = regex!(r"&").replace(&text, "").to_string();
    }

    // Parse line breaks (~)
    let tilde_count = text.chars().filter(|&c| c == '~').count();
    if tilde_count > 0 {
        result.break_line = tilde_count as u32 + 1;
    }

    result
}
