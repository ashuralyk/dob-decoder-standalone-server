use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD, Engine};
use jsonrpsee::core::async_trait;
use jsonrpsee::{proc_macros::rpc, tracing, types::error::ErrorObjectOwned};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "axum")]
use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

use crate::decoder::helpers::{decode_cluster_data, decode_spore_data};
use crate::decoder::DOBDecoder;
use crate::svg::DOBSvgExtractor;
use crate::types::Error;

// decoding result contains rendered result from native decoder and DNA string for optional use
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerDecodeResult {
    render_output: String,
    dob_content: Value,
}

#[rpc(server)]
trait DecoderRpc {
    #[method(name = "dob_protocol_version")]
    async fn protocol_versions(&self) -> Vec<String>;

    #[method(name = "dob_decode")]
    async fn decode(&self, hexed_spore_id: String) -> Result<String, ErrorObjectOwned>;

    #[method(name = "dob_batch_decode")]
    async fn batch_decode(
        &self,
        hexed_spore_ids: Vec<String>,
    ) -> Result<Vec<String>, ErrorObjectOwned>;

    #[method(name = "dob_raw_decode")]
    async fn raw_decode(
        &self,
        spore_data: String,
        cluster_data: String,
    ) -> Result<String, ErrorObjectOwned>;

    #[method(name = "dob_decode_svg")]
    async fn decode_svg(&self, hexed_spore_id: String) -> Result<String, ErrorObjectOwned>;

    #[method(name = "dob_extract_image_from_fsuri")]
    async fn extract_image_from_fsuri(
        &self,
        fsuri: String,
        encode_type: Option<String>,
    ) -> Result<String, ErrorObjectOwned>;
}

#[derive(Clone)]
pub struct DecoderStandaloneServer {
    decoder: DOBDecoder,
    cache_expiration: u64,
    svg_extractor: DOBSvgExtractor,
}

impl DecoderStandaloneServer {
    pub fn new(decoder: DOBDecoder, cache_expiration: u64) -> Self {
        Self {
            svg_extractor: DOBSvgExtractor::new(&decoder.setting().image_fetcher_url),
            decoder,
            cache_expiration,
        }
    }

    async fn cache_decode(
        &self,
        spore_id: [u8; 32],
        cache_path: PathBuf,
    ) -> Result<(String, Value), Error> {
        let (content, dna, metadata) = self.decoder.fetch_decode_ingredients(spore_id).await?;
        let render_output = self.decoder.decode_dna(&dna, metadata).await?;
        write_dob_to_cache(&render_output, &content, cache_path, self.cache_expiration)?;
        Ok((render_output, content))
    }
}

#[async_trait]
impl DecoderRpcServer for DecoderStandaloneServer {
    async fn protocol_versions(&self) -> Vec<String> {
        self.decoder.protocol_versions()
    }

    // decode DNA in particular spore DOB cell
    async fn decode(&self, hexed_spore_id: String) -> Result<String, ErrorObjectOwned> {
        tracing::info!("decoding spore_id {hexed_spore_id}");
        let spore_id: [u8; 32] = hex::decode(trim_0x(&hexed_spore_id))
            .map_err(|_| Error::HexedSporeIdParseError)?
            .try_into()
            .map_err(|_| Error::SporeIdLengthInvalid)?;
        let mut cache_path = self.decoder.setting().dobs_cache_directory.clone();
        cache_path.push(format!("{}.dob", hex::encode(spore_id)));
        let (render_output, dob_content) =
            if let Some(cache) = read_dob_from_cache(cache_path.clone(), self.cache_expiration)? {
                cache
            } else {
                self.cache_decode(spore_id, cache_path).await?
            };
        let result = serde_json::to_string(&ServerDecodeResult {
            render_output,
            dob_content,
        })
        .unwrap();
        tracing::info!("spore_id {hexed_spore_id}, result: {result}");
        Ok(result)
    }

    // decode DNA from a set
    async fn batch_decode(
        &self,
        hexed_spore_ids: Vec<String>,
    ) -> Result<Vec<String>, ErrorObjectOwned> {
        let mut await_results = Vec::new();
        for hexed_spore_id in hexed_spore_ids {
            await_results.push(self.decode(hexed_spore_id));
        }
        let results = futures::future::join_all(await_results)
            .await
            .into_iter()
            .map(|result| match result {
                Ok(result) => result,
                Err(error) => format!("server error: {error}"),
            })
            .collect();
        Ok(results)
    }

    // decode directly from spore and cluster data
    async fn raw_decode(
        &self,
        hexed_spore_data: String,
        hexed_cluster_data: String,
    ) -> Result<String, ErrorObjectOwned> {
        let spore_data =
            hex::decode(trim_0x(&hexed_spore_data)).map_err(|_| Error::SporeDataUncompatible)?;
        let cluster_data = hex::decode(trim_0x(&hexed_cluster_data))
            .map_err(|_| Error::ClusterDataUncompatible)?;
        let dob = decode_spore_data(&spore_data)?;
        let dob_metadata = decode_cluster_data(&cluster_data)?;
        let render_output = self.decoder.decode_dna(&dob.dna, dob_metadata).await?;
        let result = serde_json::to_string(&ServerDecodeResult {
            render_output,
            dob_content: dob.content,
        })
        .unwrap();
        tracing::info!("raw, result: {result}");
        Ok(result)
    }

    async fn decode_svg(&self, hexed_spore_id: String) -> Result<String, ErrorObjectOwned> {
        let ServerDecodeResult {
            render_output,
            dob_content: _,
        } = serde_json::from_str(&self.decode(hexed_spore_id).await?)
            .map_err(|e| ErrorObjectOwned::owned(-1, e.to_string(), None::<()>))?;
        let svg = self.svg_extractor.extract_svg(render_output).await?;
        Ok(svg)
    }

    async fn extract_image_from_fsuri(
        &self,
        fsuri: String,
        encode_type: Option<String>,
    ) -> Result<String, ErrorObjectOwned> {
        let raw_images = self
            .svg_extractor
            .get_fetcher()
            .fetch_images(&[fsuri])
            .await?;
        let image = raw_images.first().ok_or(Error::NoImageFound)?;

        match encode_type.as_deref() {
            Some("hex") | None => Ok(hex::encode(image)),
            Some("base64") => Ok(STANDARD.encode(image)),
            unknown => Err(ErrorObjectOwned::owned::<serde_json::Value>(
                -1,
                format!(
                    "Unknown encode type: {}. Supported types: 'base64', 'hex' (default)",
                    unknown.unwrap_or("unknown")
                ),
                None,
            )),
        }
    }
}

fn trim_0x(hexed: &str) -> &str {
    hexed.trim_start_matches("0x")
}

fn read_dob_from_cache(
    cache_path: PathBuf,
    mut expiration: u64,
) -> Result<Option<(String, Value)>, Error> {
    if !cache_path.exists() {
        return Ok(None);
    }
    let file_content = fs::read_to_string(&cache_path)
        .map_err(|_| Error::DOBRenderCacheNotFound(cache_path.clone()))?;
    let mut lines = file_content.split('\n');
    let (Some(result), Some(content), timestamp) = (lines.next(), lines.next(), lines.next())
    else {
        return Err(Error::DOBRenderCacheModified(cache_path));
    };
    if let Some(value) = timestamp {
        if !value.is_empty() {
            expiration = value
                .parse::<u64>()
                .map_err(|_| Error::DOBRenderCacheModified(cache_path.clone()))?;
        }
    }
    match serde_json::from_str(content) {
        Ok(content) => {
            if expiration > 0 && now()? > Duration::from_secs(expiration) {
                Ok(None)
            } else {
                Ok(Some((result.to_string(), content)))
            }
        }
        Err(_) => Err(Error::DOBRenderCacheModified(cache_path)),
    }
}

fn write_dob_to_cache(
    render_result: &str,
    dob_content: &Value,
    cache_path: PathBuf,
    cache_expiration: u64,
) -> Result<(), Error> {
    let expiration_timestamp = if cache_expiration > 0 {
        now()?
            .checked_add(Duration::from_secs(cache_expiration))
            .ok_or(Error::SystemTimeError)?
            .as_secs()
    } else {
        0 // zero means always read from cache
    };
    let json_dob_content = serde_json::to_string(dob_content).unwrap();
    let file_content = format!("{render_result}\n{json_dob_content}\n{expiration_timestamp}");
    fs::write(&cache_path, file_content).map_err(|_| Error::DOBRenderCacheNotFound(cache_path))?;
    Ok(())
}

fn now() -> Result<Duration, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::SystemTimeError)
}

// RESTful API implementation
#[cfg(feature = "axum")]
impl DecoderStandaloneServer {
    /// Create RESTful API routes
    pub fn create_restful_routes() -> Router<Self> {
        Router::new()
            .route("/dob_decode_svg/:spore_id", get(handle_dob_decode_svg))
            .route("/dob_decode/:spore_id", get(handle_dob_decode))
            .route("/dob_batch_decode/:spore_ids", get(handle_dob_batch_decode))
            .route(
                "/dob_raw_decode/:spore_data/:cluster_data",
                get(handle_dob_raw_decode),
            )
            .route(
                "/dob_extract_image_from_fsuri/:fsuri",
                get(handle_extract_image_from_fsuri),
            )
            .route("/protocol_versions", get(handle_protocol_versions))
    }
}

/// Handle dob_decode_svg RESTful endpoint
#[cfg(feature = "axum")]
async fn handle_dob_decode_svg(
    Path(spore_id): Path<String>,
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!("RESTful API: decoding SVG for spore_id {}", spore_id);

    match server.decode_svg(spore_id).await {
        Ok(svg_content) => {
            tracing::info!("RESTful API: SVG decoded successfully");
            (StatusCode::OK, Html(svg_content))
        }
        Err(error) => {
            tracing::error!("RESTful API: SVG decode failed: {}", error);
            (
                StatusCode::BAD_REQUEST,
                Html(format!("Error: {}", error.message())),
            )
        }
    }
}

/// Handle dob_decode RESTful endpoint
#[cfg(feature = "axum")]
async fn handle_dob_decode(
    Path(spore_id): Path<String>,
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!("RESTful API: decoding spore_id {}", spore_id);

    match server.decode(spore_id).await {
        Ok(result) => {
            tracing::info!("RESTful API: decode successful");
            (StatusCode::OK, Html(result))
        }
        Err(error) => {
            tracing::error!("RESTful API: decode failed: {}", error);
            (
                StatusCode::BAD_REQUEST,
                Html(format!("Error: {}", error.message())),
            )
        }
    }
}

/// Handle dob_batch_decode RESTful endpoint
#[cfg(feature = "axum")]
async fn handle_dob_batch_decode(
    Path(spore_ids): Path<String>,
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!("RESTful API: batch decoding spore_ids: {}", spore_ids);

    // Parse comma-separated spore IDs
    let spore_id_list: Vec<String> = spore_ids.split(',').map(|s| s.trim().to_string()).collect();

    match server.batch_decode(spore_id_list).await {
        Ok(results) => {
            tracing::info!("RESTful API: batch decode successful");
            let json_result = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
            (StatusCode::OK, Html(json_result))
        }
        Err(error) => {
            tracing::error!("RESTful API: batch decode failed: {}", error);
            (
                StatusCode::BAD_REQUEST,
                Html(format!("Error: {}", error.message())),
            )
        }
    }
}

/// Handle dob_raw_decode RESTful endpoint
#[cfg(feature = "axum")]
async fn handle_dob_raw_decode(
    Path((spore_data, cluster_data)): Path<(String, String)>,
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!(
        "RESTful API: raw decoding spore_data: {}, cluster_data: {}",
        spore_data,
        cluster_data
    );

    match server.raw_decode(spore_data, cluster_data).await {
        Ok(result) => {
            tracing::info!("RESTful API: raw decode successful");
            (StatusCode::OK, Html(result))
        }
        Err(error) => {
            tracing::error!("RESTful API: raw decode failed: {}", error);
            (
                StatusCode::BAD_REQUEST,
                Html(format!("Error: {}", error.message())),
            )
        }
    }
}

/// Handle dob_extract_image_from_fsuri RESTful endpoint
#[cfg(feature = "axum")]
async fn handle_extract_image_from_fsuri(
    Path(fsuri): Path<String>,
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!("RESTful API: extracting image from fsuri: {}", fsuri);

    match server.extract_image_from_fsuri(fsuri, None).await {
        Ok(result) => {
            tracing::info!("RESTful API: image extraction successful");
            (StatusCode::OK, Html(result))
        }
        Err(error) => {
            tracing::error!("RESTful API: image extraction failed: {}", error);
            (
                StatusCode::BAD_REQUEST,
                Html(format!("Error: {}", error.message())),
            )
        }
    }
}

/// Handle protocol_versions RESTful endpoint
#[cfg(feature = "axum")]
async fn handle_protocol_versions(
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!("RESTful API: getting protocol versions");

    let versions = server.protocol_versions().await;
    let json_result = serde_json::to_string(&versions).unwrap_or_else(|_| "[]".to_string());

    tracing::info!("RESTful API: protocol versions retrieved successfully");
    (StatusCode::OK, Html(json_result))
}
