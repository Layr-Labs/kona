//! Contains traits that describe the functionality of various data sources used in the derivation
//! pipeline's stages.

use crate::{errors::PipelineErrorKind, sources::IndexedBlobHash, types::PipelineResult};
use alloc::{boxed::Box, fmt::Debug, string::ToString, vec::Vec};
use alloy_eips::eip4844::Blob;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use core::fmt::Display;
use op_alloy_protocol::BlockInfo;

/// The BlobProvider trait specifies the functionality of a data source that can provide blobs.
#[async_trait]
pub trait BlobProvider {
    /// The error type for the [BlobProvider].
    type Error: Display + ToString + Into<PipelineErrorKind>;

    /// Fetches blobs for a given block ref and the blob hashes.
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<Box<Blob>>, Self::Error>;
}

/// The AltDAProvider trait specifies the functionality of a data source that can provide altda blobs.
#[async_trait]
pub trait AltDAProvider {
    /// The error type for the [EigenDAProvider].
    type Error: Display + ToString + Into<PipelineErrorKind>;

    async fn get_blob(&self, commitment: Bytes) -> Result<Bytes, Self::Error>;
}

/// The EigenDAProvider trait specifies the functionality of a data source that can provide eigenda blobs.
#[async_trait]
pub trait EigenDAProvider {
    /// The error type for the [EigenDAProvider].
    type Error: Display + ToString + Into<PipelineErrorKind>;

    async fn get_blob_v1(
        &self,
        batch_header_hash: Bytes,
        blob_index: u64,
    ) -> Result<Bytes, Self::Error>;

    // TODO: should this be in a separate v2 trait?
    async fn get_blob_v2(&self, blob_hashes: Bytes) -> Result<Bytes, Self::Error>;
}

/// Describes the functionality of a data source that can provide data availability information.
#[async_trait]
pub trait DataAvailabilityProvider {
    /// The item type of the data iterator.
    type Item: Send + Sync + Debug + Into<Bytes>;

    /// Returns the next data for the given [BlockInfo].
    /// Returns a `PipelineError::Eof` if there is no more data for the given block ref.
    async fn next(&mut self, block_ref: &BlockInfo) -> PipelineResult<Self::Item>;

    /// Clears the data source for the next block ref.
    fn clear(&mut self);
}
