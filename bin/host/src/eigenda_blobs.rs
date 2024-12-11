use alloy_primitives::Bytes;
use anyhow::Ok;
use kona_derive::{errors::BlobProviderError, traits::BlobProvider};
use reqwest::{header::GetAll, Client};
use tracing::trace;

/// An online implementation of the [EigenDABlobProvider] trait.
#[derive(Debug, Clone)]
pub struct OnlineEigenDABlobProvider {
    /// The base url.
    base: String,
    /// The inner reqwest client. Used to talk to proxy
    inner: Client,
}

const GET_METHOD: &str = "get";

impl OnlineEigenDABlobProvider {
    /// Creates a new instance of the [OnlineEigenDABlobProvider].
    ///
    /// The `genesis_time` and `slot_interval` arguments are _optional_ and the
    /// [OnlineEigenDABlobProvider] will attempt to load them dynamically at runtime if they are not
    /// provided.
    pub async fn new_http(base: String) -> Result<Self, anyhow::Error> {
        let inner = Client::new();
        Ok(Self { base, inner })
    }

    pub async fn fetch_eigenda_blob(
        &self,
        cert: &Bytes,
    ) -> Result<alloy_rlp::Bytes, reqwest::Error> {
        let url = format!("{}/{}/{}", self.base, GET_METHOD, cert.slice(1..));
        
        let raw_response = self.inner
            .get(url)
            .send()
            .await?;

        raw_response.bytes().await
    }

    
}