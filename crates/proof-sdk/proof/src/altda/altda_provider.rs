use alloc::boxed::Box;
use alloc::sync::Arc;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use kona_derive::traits::AltDAProvider;
use kona_preimage::CommsClient;

use crate::errors::OracleProviderError;

use super::OracleEigenDAProvider;

#[derive(Debug, Clone)]
pub struct OracleAltDAProvider<T: CommsClient> {
    /// The oracle eigenda provider.
    eigenda_provider: OracleEigenDAProvider<T>,
}

impl<T: CommsClient> OracleAltDAProvider<T> {
    /// Constructs a new oracle-backed AltDA provider.
    pub fn new(eigenda_provider: OracleEigenDAProvider<T>) -> Self {
        Self { eigenda_provider }
    }

    /// Constructs a new oracle-backed AltDA provider by constructing
    /// the respective altda providers using the oracle.
    pub fn new_from_oracle(oracle: Arc<T>) -> Self {
        Self { eigenda_provider: OracleEigenDAProvider::new(oracle) }
    }
}

#[async_trait]
impl<T: CommsClient + Send + Sync> AltDAProvider for OracleAltDAProvider<T> {
    type Error = OracleProviderError;
    /// Retrieves a blob from the oracle.
    ///
    /// ## Takes
    /// - `block_ref`: The block reference.
    /// - `blob_hash`: The blob hash.
    ///
    /// ## Returns
    /// - `Ok(blob)`: The blob.
    /// - `Err(e)`: The blob could not be retrieved.
    async fn get_blob(&self, commitment: Bytes) -> Result<Bytes, OracleProviderError> {
        todo!("implement the EigenDA blob retrieval here");
        // match commitment[1] {
        //     0 => todo!("keccak commitments are not implemented yet"),
        //     1 => match commitment[2] {
        //         // generic commitments. See https://github.com/ethereum-optimism/specs/discussions/135
        //         // for the byte -> da layer mapping.
        //         0 => self.eigenda_provider.get_blobs(commitment[3..].into()).ok(),
        //         0x0a => todo!("avail commitments are not implemented yet"),
        //         0x0c => todo!("celestia commitments are not implemented yet"),
        //         _ => return None,
        //     },
        //     _ => return None,
        // }
    }
}
