use alloc::boxed::Box;
use alloc::sync::Arc;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use kona_derive::traits::EigenDAProvider;
use kona_preimage::CommsClient;

use crate::errors::OracleProviderError;

/// The oracle-backed EigenDA provider for the client program.
#[derive(Debug, Clone)]
pub struct OracleEigenDAProvider<T: CommsClient> {
    /// The preimage oracle client.
    oracle: Arc<T>,
}

impl<T: CommsClient> OracleEigenDAProvider<T> {
    /// Constructs a new oracle-backed EigenDA provider.
    pub fn new(oracle: Arc<T>) -> Self {
        Self { oracle }
    }
}

#[async_trait]
impl<T: CommsClient + Sync + Send> EigenDAProvider for OracleEigenDAProvider<T> {
    type Error = OracleProviderError;

    async fn get_blob_v1(
        &self,
        batch_header_hash: Bytes,
        blob_index: u64,
    ) -> Result<Bytes, Self::Error> {
        todo!("implement the EigenDA blob retrieval here");
    }

    async fn get_blob_v2(&self, blob_hashes: Bytes) -> Result<Bytes, Self::Error> {
        todo!("implement the EigenDA blob retrieval here");
    }
}
