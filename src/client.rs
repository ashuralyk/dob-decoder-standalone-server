use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ckb_jsonrpc_types::{
    CellWithStatus, JsonBytes, OutPoint, TransactionWithStatusResponse, Uint32,
};
use ckb_sdk::rpc::ckb_indexer::{Cell, Order, Pagination, SearchKey, Tx};
use ckb_types::H256;
use jsonrpc_core::futures::FutureExt;
use lazy_regex::regex_replace_all;
use reqwest::{Client, ClientBuilder, Url};
use serde_json::Value;
use std::time::Duration;

use crate::types::Error;

pub type Rpc<T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'static>>;

#[allow(clippy::upper_case_acronyms)]
enum Target {
    CKB,
    Indexer,
}

macro_rules! jsonrpc {
    ($method:expr, $id:expr, $self:ident, $return:ty$(, $params:ident$(,)?)*) => {{
        let data = format!(
            r#"{{"id": {}, "jsonrpc": "2.0", "method": "{}", "params": {}}}"#,
            $self.id.load(Ordering::Relaxed),
            $method,
            serde_json::to_value(($($params,)*)).unwrap()
        );
        $self.id.fetch_add(1, Ordering::Relaxed);

        let req_json: serde_json::Value = serde_json::from_str(&data).unwrap();

        let url = match $id {
            Target::CKB => $self.ckb_uri.clone(),
            Target::Indexer => $self.indexer_uri.clone(),
        };
        let c = $self.raw.post(url).json(&req_json);
        async {
            let resp = c
                .send()
                .await
                .map_err::<Error, _>(|e| Error::JsonRpcRequestError(e.to_string()))?;
            let output = resp
                .json::<jsonrpc_core::response::Output>()
                .await
                .map_err::<Error, _>(|e| Error::JsonRpcRequestError(e.to_string()))?;

            match output {
                jsonrpc_core::response::Output::Success(success) => {
                    Ok(serde_json::from_value::<$return>(success.result).unwrap())
                }
                jsonrpc_core::response::Output::Failure(e) => {
                    Err(Error::JsonRpcRequestError(e.error.to_string()))
                }
            }
        }
    }}
}

#[derive(Clone)]
pub struct RpcClient {
    raw: Client,
    ckb_uri: Url,
    indexer_uri: Url,
    id: Arc<AtomicU64>,
}

impl RpcClient {
    pub fn new(ckb_uri: &str, indexer_uri: &str) -> Self {
        let ckb_uri = Url::parse(ckb_uri).expect("ckb uri, e.g. \"http://127.0.0.1:8114\"");
        let indexer_uri = Url::parse(indexer_uri).expect("ckb uri, e.g. \"http://127.0.0.1:8116\"");

        RpcClient {
            raw: Client::new(),
            ckb_uri,
            indexer_uri,
            id: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl RpcClient {
    pub fn get_live_cell(&self, out_point: &OutPoint, with_data: bool) -> Rpc<CellWithStatus> {
        jsonrpc!(
            "get_live_cell",
            Target::CKB,
            self,
            CellWithStatus,
            out_point,
            with_data
        )
        .boxed()
    }

    pub fn get_cells(
        &self,
        search_key: SearchKey,
        limit: u32,
        cursor: Option<JsonBytes>,
    ) -> Rpc<Pagination<Cell>> {
        let order = Order::Asc;
        let limit = Uint32::from(limit);

        jsonrpc!(
            "get_cells",
            Target::Indexer,
            self,
            Pagination<Cell>,
            search_key,
            order,
            limit,
            cursor,
        )
        .boxed()
    }

    pub fn get_transactions(
        &self,
        search_key: SearchKey,
        limit: u32,
        cursor: Option<JsonBytes>,
    ) -> Rpc<Pagination<Tx>> {
        let order = Order::Asc;
        let limit = Uint32::from(limit);

        jsonrpc!(
            "get_transactions",
            Target::Indexer,
            self,
            Pagination<Tx>,
            search_key,
            order,
            limit,
            cursor,
        )
        .boxed()
    }

    pub fn get_transaction(&self, hash: &H256) -> Rpc<Option<TransactionWithStatusResponse>> {
        jsonrpc!(
            "get_transaction",
            Target::CKB,
            self,
            Option<TransactionWithStatusResponse>,
            hash,
        )
        .boxed()
    }
}

pub struct ImageFetchClient {
    base_url: HashMap<String, Url>,
    client: Client,
}

impl ImageFetchClient {
    pub fn new(base_url: &HashMap<String, Url>) -> Self {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30)) // 30 seconds timeout
            .connect_timeout(Duration::from_secs(10)) // 10 seconds connection timeout
            .danger_accept_invalid_certs(true) // Bypass SSL certificate verification
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: base_url.clone(),
            client,
        }
    }

    pub async fn fetch_images(&self, images_uri: &[String]) -> Result<Vec<Vec<u8>>, Error> {
        let mut requests = vec![];
        for uri in images_uri {
            match uri.try_into()? {
                URI::BTCFS(tx_hash, index) => {
                    let url = self
                        .base_url
                        .get("btcfs")
                        .ok_or(Error::FsuriNotFoundInConfig)?
                        .join(&tx_hash)
                        .expect("image url");
                    requests.push(parse_image_from_btcfs(url, index).boxed());
                }
                URI::IPFS(cid) => {
                    let url = self
                        .base_url
                        .get("ipfs")
                        .ok_or(Error::FsuriNotFoundInConfig)?
                        .join(&cid)
                        .expect("image url");
                    requests.push(
                        async move {
                            let image = reqwest::get(url.clone())
                                .await
                                .map_err(|e| Error::FetchFromIpfsError(e.to_string()))?
                                .bytes()
                                .await
                                .map_err(|e| Error::FetchFromIpfsError(e.to_string()))?
                                .to_vec();
                            Ok(image)
                        }
                        .boxed(),
                    );
                }
                URI::Http(url) => {
                    let client = self.client.clone();
                    requests.push(
                        async move {
                            let response = client.get(url.clone()).send().await.map_err(|e| {
                                Error::FetchFromHttpError(format!("HTTP request failed: {}", e))
                            })?;

                            if !response.status().is_success() {
                                return Err(Error::FetchFromHttpError(format!(
                                    "HTTP request failed with status: {}",
                                    response.status()
                                )));
                            }

                            let image = response
                                .bytes()
                                .await
                                .map_err(|e| {
                                    Error::FetchFromHttpError(format!(
                                        "Failed to read response body: {}",
                                        e
                                    ))
                                })?
                                .to_vec();
                            Ok(image)
                        }
                        .boxed(),
                    );
                }
                URI::Https(url) => {
                    let client = self.client.clone();
                    requests.push(
                        async move {
                            let response = client.get(url.clone()).send().await.map_err(|e| {
                                let error_msg = if e.is_connect() {
                                    format!("HTTPS connection failed: {}", e)
                                } else if e.is_timeout() {
                                    format!("HTTPS request timeout: {}", e)
                                } else if e.is_request() {
                                    format!("HTTPS request error: {}", e)
                                } else {
                                    format!("HTTPS error: {}", e)
                                };
                                Error::FetchFromHttpError(error_msg)
                            })?;

                            if !response.status().is_success() {
                                return Err(Error::FetchFromHttpError(format!(
                                    "HTTPS request failed with status: {}",
                                    response.status()
                                )));
                            }

                            let image = response
                                .bytes()
                                .await
                                .map_err(|e| {
                                    Error::FetchFromHttpError(format!(
                                        "Failed to read HTTPS response body: {}",
                                        e
                                    ))
                                })?
                                .to_vec();
                            Ok(image)
                        }
                        .boxed(),
                    );
                }
            }
        }
        let mut images = vec![];
        let responses = futures::future::join_all(requests).await;
        for response in responses {
            images.push(response?);
        }
        Ok(images)
    }
}

#[allow(clippy::upper_case_acronyms)]
enum URI {
    BTCFS(String, usize),
    IPFS(String),
    Http(String),
    Https(String),
}

impl TryFrom<&String> for URI {
    type Error = Error;

    fn try_from(uri: &String) -> Result<Self, Error> {
        if let Some(body) = uri.strip_prefix("btcfs://") {
            let parts: Vec<&str> = body.split('i').collect::<Vec<_>>();
            if parts.len() != 2 {
                return Err(Error::InvalidOnchainFsuriFormat);
            }
            let tx_hash = parts[0].to_string();
            let index = parts[1]
                .parse()
                .map_err(|_| Error::InvalidOnchainFsuriFormat)?;
            Ok(URI::BTCFS(tx_hash, index))
        } else if let Some(body) = uri.strip_prefix("ipfs://") {
            let hash = body.to_string();
            Ok(URI::IPFS(hash))
        } else if uri.starts_with("https") {
            Ok(URI::Https(uri.clone()))
        } else if uri.starts_with("http") {
            Ok(URI::Http(uri.clone()))
        } else {
            Err(Error::InvalidOnchainFsuriFormat)
        }
    }
}

async fn parse_image_from_btcfs(url: Url, index: usize) -> Result<Vec<u8>, Error> {
    // parse btc transaction
    let btc_tx = reqwest::get(url)
        .await
        .map_err(|e| Error::FetchFromBtcNodeError(e.to_string()))?
        .json::<Value>()
        .await
        .map_err(|e| Error::FetchFromBtcNodeError(e.to_string()))?;
    let vin = btc_tx
        .get("vin")
        .ok_or(Error::InvalidBtcTransactionFormat(
            "vin not found".to_string(),
        ))?
        .as_array()
        .ok_or(Error::InvalidBtcTransactionFormat(
            "vin not an array".to_string(),
        ))?
        .first()
        .ok_or(Error::InvalidBtcTransactionFormat(
            "vin is empty".to_string(),
        ))?;
    let witness = vin
        .get("inner_witnessscript_asm")
        .ok_or(Error::InvalidBtcTransactionFormat(
            "inner_witnessscript_asm not found".to_string(),
        ))?
        .as_str()
        .ok_or(Error::InvalidBtcTransactionFormat(
            "inner_witnessscript_asm not a string".to_string(),
        ))?
        .to_owned();

    // parse inscription body
    let mut images = vec![];
    let mut witness_view = witness.as_str();
    const HEADER: &str = "OP_IF OP_PUSHBYTES_3 444f42 OP_PUSHBYTES_1 01 OP_PUSHBYTES_9 696d6167652f706e67 OP_0 OP_PUSHDATA2 ";
    while let (Some(start), Some(end)) = (witness_view.find("OP_IF"), witness_view.find("OP_ENDIF"))
    {
        if start >= end {
            return Err(Error::InvalidInscriptionFormat);
        }
        let inscription = &witness_view[start..end + "OP_ENDIF".len()];
        if !inscription.contains(HEADER) {
            return Err(Error::InvalidInscriptionFormat);
        }
        let base_removed = inscription.replace(HEADER, "");
        let hexed = regex_replace_all!(r#"\s?OP\_\w+\s?"#, &base_removed, "");
        let image =
            hex::decode(hexed.as_bytes()).map_err(|_| Error::InvalidInscriptionContentHexFormat)?;
        images.push(image);
        witness_view = &witness_view[end + "OP_ENDIF".len()..];
    }
    if images.is_empty() {
        return Err(Error::EmptyInscriptionContent);
    }

    let image = images
        .get(index)
        .cloned()
        .ok_or(Error::ExceededInscriptionIndex)?;
    Ok(image)
}
