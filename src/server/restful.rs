use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use jsonrpsee::tracing;

use crate::server::DecoderStandaloneServer;

// RESTful API implementation
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
async fn handle_dob_decode_svg(
    Path(spore_id): Path<String>,
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!("RESTful API: decoding SVG for spore_id {}", spore_id);

    match server.service_decode_svg(spore_id).await {
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
async fn handle_dob_decode(
    Path(spore_id): Path<String>,
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!("RESTful API: decoding spore_id {}", spore_id);

    match server.service_decode(spore_id).await {
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
async fn handle_dob_batch_decode(
    Path(spore_ids): Path<String>,
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!("RESTful API: batch decoding spore_ids: {}", spore_ids);

    // Parse comma-separated spore IDs
    let spore_id_list: Vec<String> = spore_ids.split(',').map(|s| s.trim().to_string()).collect();

    match server.service_batch_decode(spore_id_list).await {
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
async fn handle_dob_raw_decode(
    Path((spore_data, cluster_data)): Path<(String, String)>,
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!(
        "RESTful API: raw decoding spore_data: {}, cluster_data: {}",
        spore_data,
        cluster_data
    );

    match server.service_raw_decode(spore_data, cluster_data).await {
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
async fn handle_extract_image_from_fsuri(
    Path(fsuri): Path<String>,
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!("RESTful API: extracting image from fsuri: {}", fsuri);

    match server.service_extract_image_from_fsuri(fsuri, None).await {
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
async fn handle_protocol_versions(
    axum::extract::State(server): axum::extract::State<DecoderStandaloneServer>,
) -> impl IntoResponse {
    tracing::info!("RESTful API: getting protocol versions");

    let versions = server.service_protocol_versions().await;
    let json_result = serde_json::to_string(&versions).unwrap_or_else(|_| "[]".to_string());

    tracing::info!("RESTful API: protocol versions retrieved successfully");
    (StatusCode::OK, Html(json_result))
}
