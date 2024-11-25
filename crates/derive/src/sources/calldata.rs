//! CallData Source

use crate::{
    errors::PipelineError,
    traits::{AltDAProvider, ChainProvider, DataAvailabilityProvider},
    types::PipelineResult,
};
use alloc::{boxed::Box, collections::VecDeque};
use alloy_consensus::{Transaction, TxEnvelope};
use alloy_primitives::{Address, Bytes};
use async_trait::async_trait;
use op_alloy_protocol::BlockInfo;

use super::altda_data::BatcherSubmission;

/// A data iterator that reads from calldata.
#[derive(Debug, Clone)]
pub struct CalldataSource<CP, AP>
where
    CP: ChainProvider + Send,
    AP: AltDAProvider + Send,
{
    /// The chain provider to use for the calldata source.
    pub chain_provider: CP,
    /// The altda provider to use to fetch blobs when the calldata contains an altda commitment.
    pub altda_provider: Option<AP>,
    /// The batch inbox address.
    pub batch_inbox_address: Address,
    /// The L1 Signer.
    pub signer: Address,
    /// Current calldata.
    pub calldata: VecDeque<Bytes>,
    /// Whether the calldata source is open.
    pub open: bool,
}

impl<CP, AP> CalldataSource<CP, AP>
where
    CP: ChainProvider + Send,
    AP: AltDAProvider + Send,
{
    /// Creates a new calldata source.
    pub const fn new(
        chain_provider: CP,
        altda_provider: Option<AP>,
        batch_inbox_address: Address,
        signer: Address,
    ) -> Self {
        Self {
            chain_provider,
            altda_provider,
            batch_inbox_address,
            signer,
            calldata: VecDeque::new(),
            open: false,
        }
    }

    /// Loads the calldata into the source if it is not open.
    async fn load_calldata(&mut self, block_ref: &BlockInfo) -> Result<(), CP::Error> {
        if self.open {
            return Ok(());
        }

        let (_, txs) =
            self.chain_provider.block_info_and_transactions_by_hash(block_ref.hash).await?;

        let data_or_commitments = txs
            .iter()
            .filter_map(|tx| {
                let (tx_kind, data) = match tx {
                    TxEnvelope::Legacy(tx) => (tx.tx().to(), tx.tx().input()),
                    TxEnvelope::Eip2930(tx) => (tx.tx().to(), tx.tx().input()),
                    TxEnvelope::Eip1559(tx) => (tx.tx().to(), tx.tx().input()),
                    _ => return None,
                };
                let to = tx_kind?;

                if to != self.batch_inbox_address {
                    return None;
                }
                if tx.recover_signer().ok()? != self.signer {
                    return None;
                }
                Some(data.to_vec().into())
            })
            .collect::<VecDeque<Bytes>>();

        // TODO: refactor this to use an async filter_map to fit in previous filter_map
        let mut results = VecDeque::new();
        for data_or_commitment in data_or_commitments {
            // use parse() to determine the type of commitment
            let submission = BatcherSubmission::parse(data_or_commitment.clone());
            let data = match submission {
                None => continue,
                // return data_or_commitment (including version byte), because frame queue expects it
                Some(BatcherSubmission::Frames(_)) => data_or_commitment,
                Some(BatcherSubmission::Commitment(altda_commitment)) => {
                    let provider = if let Some(p) = self.altda_provider.as_ref() {
                        p
                    } else {
                        warn!("altda commitment found but no altda provider is set");
                        continue;
                    };
                    match provider.get_blob(altda_commitment).await {
                        Ok(blob) => blob,
                        Err(err) => {
                            warn!("failed to fetch altda commitment: {}", err);
                            continue;
                        }
                    }
                }
            };
            results.push_back(data);
        }
        self.calldata = results;
        self.open = true;

        Ok(())
    }
}

#[async_trait]
impl<CP, AP> DataAvailabilityProvider for CalldataSource<CP, AP>
where
    CP: ChainProvider + Send,
    AP: AltDAProvider + Send,
{
    type Item = Bytes;

    async fn next(&mut self, block_ref: &BlockInfo) -> PipelineResult<Self::Item> {
        self.load_calldata(block_ref).await.map_err(Into::into)?;
        self.calldata.pop_front().ok_or(PipelineError::Eof.temp())
    }

    fn clear(&mut self) {
        self.calldata.clear();
        self.open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        errors::PipelineErrorKind,
        sources::altda_data::DERIVATION_VERSION_1,
        test_utils::{TestAltDAProvider, TestChainProvider},
    };
    use alloc::{vec, vec::Vec};
    use alloy_consensus::{Signed, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxLegacy};
    use alloy_primitives::{address, Address, PrimitiveSignature as Signature, TxKind};

    pub(crate) fn init_test_logging() {
        use tracing_subscriber::layer::SubscriberExt;
        let subscriber =
            tracing_subscriber::Registry::default().with(tracing_subscriber::fmt::Layer::default());
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");
    }

    pub(crate) fn test_legacy_tx(to: Address) -> TxEnvelope {
        let sig = Signature::test_signature();
        TxEnvelope::Legacy(Signed::new_unchecked(
            TxLegacy { to: TxKind::Call(to), ..Default::default() },
            sig,
            Default::default(),
        ))
    }

    pub(crate) fn test_eip2930_tx(to: Address) -> TxEnvelope {
        let sig = Signature::test_signature();
        TxEnvelope::Eip2930(Signed::new_unchecked(
            TxEip2930 { to: TxKind::Call(to), ..Default::default() },
            sig,
            Default::default(),
        ))
    }

    pub(crate) fn test_eip1559_tx(to: Address, input: Bytes) -> TxEnvelope {
        let sig = Signature::test_signature();
        TxEnvelope::Eip1559(Signed::new_unchecked(
            TxEip1559 { to: TxKind::Call(to), input, ..Default::default() },
            sig,
            Default::default(),
        ))
    }

    pub(crate) fn test_blob_tx(to: Address) -> TxEnvelope {
        let sig = Signature::test_signature();
        TxEnvelope::Eip4844(Signed::new_unchecked(
            TxEip4844Variant::TxEip4844(TxEip4844 { to, ..Default::default() }),
            sig,
            Default::default(),
        ))
    }

    pub(crate) fn default_test_calldata_source(
    ) -> CalldataSource<TestChainProvider, TestAltDAProvider> {
        CalldataSource::new(
            TestChainProvider::default(),
            Some(TestAltDAProvider::default()),
            Default::default(),
            Default::default(),
        )
    }

    #[tokio::test]
    async fn test_clear_calldata() {
        let mut source = default_test_calldata_source();
        source.open = true;
        source.calldata.push_back(Bytes::default());
        source.clear();
        assert!(source.calldata.is_empty());
        assert!(!source.open);
    }

    #[tokio::test]
    async fn test_load_calldata_open() {
        let mut source = default_test_calldata_source();
        source.open = true;
        assert!(source.load_calldata(&BlockInfo::default()).await.is_ok());
    }

    #[tokio::test]
    async fn test_load_calldata_provider_err() {
        let mut source = default_test_calldata_source();
        assert!(source.load_calldata(&BlockInfo::default()).await.is_err());
    }

    #[tokio::test]
    async fn test_load_calldata_chain_provider_empty_txs() {
        let mut source = default_test_calldata_source();
        let block_info = BlockInfo::default();
        source.chain_provider.insert_block_with_transactions(0, block_info, Vec::new());
        assert!(!source.open); // Source is not open by default.
        assert!(source.load_calldata(&BlockInfo::default()).await.is_ok());
        assert!(source.calldata.is_empty());
        assert!(source.open);
    }

    #[tokio::test]
    async fn test_load_calldata_wrong_batch_inbox_address() {
        let batch_inbox_address = address!("0123456789012345678901234567890123456789");
        let mut source = default_test_calldata_source();
        let block_info = BlockInfo::default();
        let tx = test_legacy_tx(batch_inbox_address);
        source.chain_provider.insert_block_with_transactions(0, block_info, vec![tx]);
        assert!(!source.open); // Source is not open by default.
        assert!(source.load_calldata(&BlockInfo::default()).await.is_ok());
        assert!(source.calldata.is_empty());
        assert!(source.open);
    }

    #[tokio::test]
    async fn test_load_calldata_wrong_signer() {
        let batch_inbox_address = address!("0123456789012345678901234567890123456789");
        let mut source = default_test_calldata_source();
        source.batch_inbox_address = batch_inbox_address;
        let block_info = BlockInfo::default();
        let tx = test_legacy_tx(batch_inbox_address);
        source.chain_provider.insert_block_with_transactions(0, block_info, vec![tx]);
        assert!(!source.open); // Source is not open by default.
        assert!(source.load_calldata(&BlockInfo::default()).await.is_ok());
        assert!(source.calldata.is_empty());
        assert!(source.open);
    }

    #[tokio::test]
    async fn test_load_calldata_valid_legacy_tx() {
        let batch_inbox_address = address!("0123456789012345678901234567890123456789");
        let mut source = default_test_calldata_source();
        source.batch_inbox_address = batch_inbox_address;
        let tx = test_legacy_tx(batch_inbox_address);
        source.signer = tx.recover_signer().unwrap();
        let block_info = BlockInfo::default();
        source.chain_provider.insert_block_with_transactions(0, block_info, vec![tx]);
        assert!(!source.open); // Source is not open by default.
        assert!(source.load_calldata(&BlockInfo::default()).await.is_ok());
        assert!(!source.calldata.is_empty()); // Calldata is NOT empty.
        assert!(source.open);
    }

    #[tokio::test]
    async fn test_load_calldata_valid_eip2930_tx() {
        let batch_inbox_address = address!("0123456789012345678901234567890123456789");
        let mut source = default_test_calldata_source();
        source.batch_inbox_address = batch_inbox_address;
        let tx = test_eip2930_tx(batch_inbox_address);
        source.signer = tx.recover_signer().unwrap();
        let block_info = BlockInfo::default();
        source.chain_provider.insert_block_with_transactions(0, block_info, vec![tx]);
        assert!(!source.open); // Source is not open by default.
        assert!(source.load_calldata(&BlockInfo::default()).await.is_ok());
        assert!(!source.calldata.is_empty()); // Calldata is NOT empty.
        assert!(source.open);
    }

    #[tokio::test]
    async fn test_load_calldata_valid_altda_tx() {
        let batch_inbox_address = address!("0123456789012345678901234567890123456789");
        let mut source = default_test_calldata_source();
        source.batch_inbox_address = batch_inbox_address;
        let altda_commitment_tx_input = Bytes::from([DERIVATION_VERSION_1, 1, 2, 3]);
        let tx = test_eip1559_tx(batch_inbox_address, altda_commitment_tx_input);
        source.signer = tx.recover_signer().unwrap();
        let block_info = BlockInfo::default();
        source.chain_provider.insert_block_with_transactions(0, block_info, vec![tx]);
        source
            .altda_provider
            .as_mut()
            .unwrap()
            .insert_blob(Bytes::from([1, 2, 3]), Bytes::from([4, 5, 6]));
        assert!(!source.open); // Source is not open by default.
        assert!(source.load_calldata(&BlockInfo::default()).await.is_ok());
        assert!(!source.calldata.is_empty()); // Calldata is NOT empty.
        assert!(source.open);
    }

    #[tokio::test]
    async fn test_load_calldata_blob_tx_ignored() {
        let batch_inbox_address = address!("0123456789012345678901234567890123456789");
        let mut source = default_test_calldata_source();
        source.batch_inbox_address = batch_inbox_address;
        let tx = test_blob_tx(batch_inbox_address);
        source.signer = tx.recover_signer().unwrap();
        let block_info = BlockInfo::default();
        source.chain_provider.insert_block_with_transactions(0, block_info, vec![tx]);
        assert!(!source.open); // Source is not open by default.
        assert!(source.load_calldata(&BlockInfo::default()).await.is_ok());
        assert!(source.calldata.is_empty());
        assert!(source.open);
    }

    #[tokio::test]
    async fn test_next_err_loading_calldata() {
        let mut source = default_test_calldata_source();
        assert!(matches!(
            source.next(&BlockInfo::default()).await,
            Err(PipelineErrorKind::Temporary(_))
        ));
    }
}
