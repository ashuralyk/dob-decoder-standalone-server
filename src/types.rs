use std::path::PathBuf;

use ckb_types::H256;
use serde::Deserialize;

#[cfg(feature = "standalone_server")]
use jsonrpsee::types::ErrorCode;
#[cfg(feature = "standalone_server")]
use serde::Serialize;

#[derive(thiserror::Error, Debug)]
#[repr(i32)]
pub enum Error {
    #[error("DNA bytes length not match the requirement in Cluster")]
    DnaLengthNotMatch = 1001,
    #[error("spore id length should equal to 32")]
    SporeIdLengthInvalid,
    #[error("natvie decoder not found")]
    NativeDecoderNotFound,
    #[error("spore id not exist on-chain")]
    SporeIdNotFound,
    #[error("uncompatible spore data")]
    SporeDataUncompatible,
    #[error("uncompatible spore data content_type")]
    SporeDataContentTypeUncompatible,
    #[error("unexpected DOB protocol version")]
    DOBVersionUnexpected,
    #[error("miss cluster id in spore data")]
    ClusterIdNotSet,
    #[error("cluster id not exist on-chain")]
    ClusterIdNotFound,
    #[error("uncompatible cluster data")]
    ClusterDataUncompatible,
    #[error("decoder id not exist on-chain")]
    DecoderIdNotFound,
    #[error("output of decoder should contain at least one line")]
    DecoderOutputInvalid,
    #[error("DNA string is not in hex format")]
    HexedDNAParseError,
    #[error("spore id string is not in hex format")]
    HexedSporeIdParseError,
    #[error("invalid decoder path to persist")]
    DecoderBinaryPathInvalid,
    #[error("encounter error while executing DNA decoding")]
    DecoderExecutionError,
    #[error("decoding program triggered an error")]
    DecoderExecutionInternalError,
    #[error("encounter error while searching live cells")]
    FetchLiveCellsError,
    #[error("spore content cannot parse to DOB content")]
    DOBContentUnexpected,
    #[error("cluster description cannot parse to DOB metadata")]
    DOBMetadataUnexpected,
}

#[cfg(feature = "standalone_server")]
impl From<Error> for ErrorCode {
    fn from(value: Error) -> Self {
        (value as i32).into()
    }
}

// value on `description` field in Cluster data, adapting for DOB protocol in JSON format
#[derive(Deserialize)]
pub struct ClusterDescriptionField {
    pub description: String,
    pub dob: DOBClusterFormat,
}

// contains `decoder` and `pattern` identifiers
#[derive(Deserialize)]
pub struct DOBClusterFormat {
    pub decoder: DOBDecoderFormat,
    pub pattern: String,
    pub dna_bytes: u8,
}

// restricted decoder locator type
#[derive(Deserialize)]
pub enum DecoderLocationType {
    #[serde(alias = "type_id")]
    TypeId,
    #[serde(alias = "code_hash")]
    CodeHash,
}

// decoder location information
#[derive(Deserialize)]
pub struct DOBDecoderFormat {
    #[serde(alias = "type")]
    pub location: DecoderLocationType,
    pub hash: H256,
}

// value on `content` field in Spore data, adapting for DOB protocol in JSON format
#[derive(Deserialize)]
#[cfg_attr(feature = "standalone_server", derive(Serialize, Clone))]
pub struct SporeContentField {
    pub block_number: u64,
    pub cell_id: u64,
    pub dna: String,
}

// standalone server settings in TOML format
#[cfg_attr(feature = "standalone_server", derive(Serialize, Deserialize))]
#[derive(Default)]
pub struct Settings {
    pub protocol_version: String,
    pub ckb_rpc: String,
    pub rpc_server_address: String,
    pub ckb_vm_runner: String,
    pub decoders_cache_directory: PathBuf,
    pub avaliable_spore_code_hashes: Vec<H256>,
    pub avaliable_cluster_code_hashes: Vec<H256>,
}

// decoding result contains rendered result from native decoder and DNA string for optional use
#[cfg(feature = "standalone_server")]
#[derive(Serialize, Clone)]
pub struct ServerDecodeResult {
    pub raw_render_result: String,
    pub dob_content: SporeContentField,
}
