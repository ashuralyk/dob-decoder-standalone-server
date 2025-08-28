use std::collections::HashMap;

use base64::{engine::general_purpose::STANDARD, Engine};
use lazy_regex::regex;
use reqwest::Url;
use serde_json::Value;

use crate::{
    client::ImageFetchClient,
    svg::puretext::{
        parsers::{dob_output_parser, render_text_params_parser},
        render::render_text_parser_result_to_svg,
    },
    types::{Error, StandardDOBOutput},
};

pub mod puretext;

const DOB0_TRAIT_NAME: &str = "prev.bg";
const DOB1_TRAIT_NAME: &str = "IMAGE";
const DEFAULT_SIZE: u32 = 500;

/// Detects the MIME type of an image from its hex-encoded content by examining file signatures.
/// Returns Some(mime_type) if recognized, or None if not recognized.
pub fn detect_image_mime_type(hex_content: String) -> Option<&'static str> {
    // Skip if string is too short to contain a signature and content
    if hex_content.len() < 64 {
        return None;
    }

    // Extract just the file header (first 32 bytes should be enough for most formats)
    // and convert to lowercase for consistent comparison
    let header = &hex_content[..64].to_ascii_lowercase();

    // JPEG: starts with ffd8ff
    if header.starts_with("ffd8ff") {
        return Some("image/jpeg");
    }

    // PNG: starts with 89504e47 (‰PNG)
    if header.starts_with("89504e47") {
        return Some("image/png");
    }

    // GIF: starts with 474946 (GIF)
    if header.starts_with("474946") {
        return Some("image/gif");
    }

    // WebP: RIFF....WEBP
    if header.starts_with("52494646") && header.get(16..24) == Some("57454250") {
        return Some("image/webp");
    }

    // BMP: starts with 424d (BM)
    if header.starts_with("424d") {
        return Some("image/bmp");
    }

    // SVG: starts with <svg or <?xml
    if header.starts_with("3c737667") || header.starts_with("3c3f786d6c") {
        return Some("image/svg+xml");
    }

    // TIFF: starts with 49492a00 (Intel) or 4d4d002a (Motorola)
    if header.starts_with("49492a00") || header.starts_with("4d4d002a") {
        return Some("image/tiff");
    }

    // ICO: starts with 00000100
    if header.starts_with("00000100") {
        return Some("image/x-icon");
    }

    // AVIF: ftyp....avif
    if hex_content.to_ascii_lowercase().contains("66747970")
        && hex_content.to_ascii_lowercase().contains("61766966")
    {
        return Some("image/avif");
    }

    None
}

pub struct DOBSvgExtractor {
    fetcher: ImageFetchClient,
}

impl DOBSvgExtractor {
    pub fn new(base_url: &HashMap<String, Url>) -> Self {
        Self {
            fetcher: ImageFetchClient::new(base_url),
        }
    }

    pub fn get_fetcher(&self) -> &ImageFetchClient {
        &self.fetcher
    }

    pub async fn extract_svg(&self, dob_render_output: String) -> Result<String, Error> {
        let parsed_dob: Vec<StandardDOBOutput> =
            serde_json::from_str(&dob_render_output).map_err(|_| Error::DOBRenderOutputInvalid)?;

        if let Some(dob0_svg) = self.extract_png_svg_from_dob0(&parsed_dob).await? {
            return Ok(dob0_svg);
        }

        if let Some(dob1_svg) = self.extract_png_svg_from_dob1(&parsed_dob).await? {
            return Ok(dob1_svg);
        }

        self.extract_text_svg_from_dob0(&parsed_dob)
    }

    async fn extract_png_svg_from_dob0(
        &self,
        parsed_dob: &[StandardDOBOutput],
    ) -> Result<Option<String>, Error> {
        let fsurl = parsed_dob.iter().find_map(|dob| {
            if dob.name == DOB0_TRAIT_NAME {
                if let Some(dob_trait) = dob.traits.iter().find(|value| value.type_ == "String") {
                    if let Value::String(fsurl) = &dob_trait.value {
                        if fsurl.starts_with("btcfs://") || fsurl.starts_with("ipfs://") {
                            return Some(fsurl);
                        }
                    }
                }
            }
            None
        });
        if let Some(dob0_fsurl) = fsurl {
            let image_content = self
                .fetcher
                .fetch_images(&[dob0_fsurl.clone()])
                .await?
                .into_iter()
                .next()
                .ok_or(Error::FetchFromIpfsError("No image found".to_string()))?;
            let Some(image_mime_type) = detect_image_mime_type(hex::encode(&image_content)) else {
                return Ok(None);
            };
            let image_content_base64 = STANDARD.encode(&image_content);
            let svg_content = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?><svg xmlns="http://www.w3.org/2000/svg" width="500" height="500" viewBox="0 0 500 500" version="1.1"><image width="{DEFAULT_SIZE}" height="{DEFAULT_SIZE}" href="data:{image_mime_type};base64,{image_content_base64}" preserveAspectRatio="xMidYMid slice" /></svg>"#
            );
            Ok(Some(svg_content))
        } else {
            Ok(None)
        }
    }

    async fn extract_png_svg_from_dob1(
        &self,
        parsed_dob: &[StandardDOBOutput],
    ) -> Result<Option<String>, Error> {
        let svg = parsed_dob.iter().find_map(|dob| {
            if dob.name == DOB1_TRAIT_NAME {
                if let Some(dob_trait) = dob.traits.iter().find(|value| value.type_ == "SVG") {
                    if let Value::String(svg) = &dob_trait.value {
                        return Some(svg);
                    }
                }
            }
            None
        });
        if let Some(svg) = svg {
            Ok(self.replace_svg_fsurls(svg.clone()).await?)
        } else {
            Ok(None)
        }
    }

    fn extract_text_svg_from_dob0(
        &self,
        parsed_dob: &[StandardDOBOutput],
    ) -> Result<String, Error> {
        let dob_output_result = dob_output_parser(parsed_dob);
        let text_render_result = render_text_params_parser(
            &dob_output_result.traits,
            &dob_output_result.index_var_register,
            None,
        );
        let svg = render_text_parser_result_to_svg(&text_render_result);
        Ok(svg)
    }

    async fn replace_svg_fsurls(&self, mut svg_content: String) -> Result<Option<String>, Error> {
        // Create regex patterns to match btcfs:// and ipfs:// URLs in href attributes
        let btcfs_pattern = regex!(r#"href='btcfs://([^']+)'"#);
        let ipfs_pattern = regex!(r#"href='ipfs://([^']+)'"#);

        // Find all btcfs URLs
        let btcfs_urls: Vec<String> = btcfs_pattern
            .captures_iter(&svg_content)
            .map(|cap| format!("btcfs://{}", &cap[1]))
            .collect();

        // Find all ipfs URLs
        let ipfs_urls: Vec<String> = ipfs_pattern
            .captures_iter(&svg_content)
            .map(|cap| format!("ipfs://{}", &cap[1]))
            .collect();

        // Combine all URLs to fetch
        let mut all_urls = btcfs_urls;
        all_urls.extend(ipfs_urls);

        if all_urls.is_empty() {
            return Ok(None);
        }

        // Fetch all images
        let image_contents = self.fetcher.fetch_images(&all_urls).await?;

        // Replace URLs with base64 content
        for (i, url) in all_urls.iter().enumerate() {
            let image_content = &image_contents[i];
            let Some(mime_type) = detect_image_mime_type(hex::encode(image_content)) else {
                continue;
            };
            let base64_content = STANDARD.encode(image_content);
            let data_url = format!("data:{};base64,{}", mime_type, base64_content);

            // Replace the URL in the SVG
            let old_href = format!("href='{}'", url);
            let new_href = format!("href='{}'", data_url);
            svg_content = svg_content.replace(&old_href, &new_href);
        }

        Ok(Some(svg_content))
    }
}
