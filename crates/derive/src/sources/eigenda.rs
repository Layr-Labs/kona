//! Contains the [EigenDADataSource], which is a concrete implementation of the
//! [DataAvailabilityProvider] trait for the EigenDA protocol.
use crate::{
    sources::{EigenDABlobSource, BlobSource, CalldataSource, EthereumDataSource},
    traits::{EigenDABlobProvider, BlobProvider, ChainProvider, DataAvailabilityProvider},
    types::PipelineResult,
};
use alloc::{boxed::Box, fmt::Debug};
use alloy_primitives::Bytes;
use async_trait::async_trait;
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::BlockInfo;

/// A factory for creating an Ethereum data source provider.
#[derive(Debug, Clone)]
pub struct EigenDADataSource<C, B, A>
where
    C: ChainProvider + Send + Clone,
    B: BlobProvider + Send + Clone,
    A: EigenDABlobProvider + Send + Clone,
{
    /// The blob source.
    pub ethereum_source: EthereumDataSource<C, B>,
    /// The eigenda source.
    pub eigenda_source: EigenDABlobSource<A>,
}

impl<C, B, A> EigenDADataSource<C, B, A>
where
    C: ChainProvider + Send + Clone + Debug,
    B: BlobProvider + Send + Clone + Debug,
    A: EigenDABlobProvider + Send + Clone + Debug,
{
    /// Instantiates a new [EigenDADataSource].
    pub const fn new(
        ethereum_source: EthereumDataSource<C, B>,
        eigenda_source: EigenDABlobSource<A>,
    ) -> Self {
        Self { ethereum_source, eigenda_source }
    }
}

#[async_trait]
impl<C, B, A> DataAvailabilityProvider for EigenDADataSource<C, B, A>
where
    C: ChainProvider + Send + Sync + Clone + Debug,
    B: BlobProvider + Send + Sync + Clone + Debug,
    A: EigenDABlobProvider + Send + Sync + Clone + Debug,
{
    type Item = Bytes;

    async fn next(&mut self, block_ref: &BlockInfo) -> PipelineResult<Self::Item> {
        // then acutally use ethereum da to fetch. items are Bytes
        let item = self.ethereum_source.next(block_ref).await?;

        // just dump all the data out
        info!(target: "eth-datasource", "next item {:?}", item);

        let eigenda_source_result = self.eigenda_source.next(&item).await;
        info!(target: "eigenda-datasource", "eigenda_source_result {:?}", eigenda_source_result);
        eigenda_source_result
    } 

    fn clear(&mut self) {
        self.eigenda_source.clear();
        self.ethereum_source.clear();
    }
}