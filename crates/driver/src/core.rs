//! The driver of the kona derivation pipeline.

use alloc::vec::Vec;
use alloy_consensus::{BlockBody, Sealable};
use alloy_primitives::B256;
use alloy_rlp::Decodable;
use core::fmt::Debug;
use kona_derive::{
    errors::{PipelineError, PipelineErrorKind},
    traits::{Pipeline, SignalReceiver},
    types::Signal,
};
use op_alloy_consensus::{OpBlock, OpTxEnvelope, OpTxType};
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::L2BlockInfo;
use op_alloy_rpc_types_engine::OpAttributesWithParent;

use crate::{
    DriverError, DriverPipeline, DriverResult, Executor, ExecutorConstructor, PipelineCursor,
    TipCursor,
};

/// The Rollup Driver entrypoint.
#[derive(Debug)]
pub struct Driver<E, EC, DP, P>
where
    E: Executor + Send + Sync + Debug,
    EC: ExecutorConstructor<E> + Send + Sync + Debug,
    DP: DriverPipeline<P> + Send + Sync + Debug,
    P: Pipeline + SignalReceiver + Send + Sync + Debug,
{
    /// Marker for the executor.
    _marker: core::marker::PhantomData<E>,
    /// Marker for the pipeline.
    _marker2: core::marker::PhantomData<P>,
    /// A pipeline abstraction.
    pipeline: DP,
    /// Cursor to keep track of the L2 tip
    cursor: PipelineCursor,
    /// Executor constructor.
    executor: EC,
}

impl<E, EC, DP, P> Driver<E, EC, DP, P>
where
    E: Executor + Send + Sync + Debug,
    EC: ExecutorConstructor<E> + Send + Sync + Debug,
    DP: DriverPipeline<P> + Send + Sync + Debug,
    P: Pipeline + SignalReceiver + Send + Sync + Debug,
{
    /// Creates a new [Driver].
    pub const fn new(cursor: PipelineCursor, executor: EC, pipeline: DP) -> Self {
        Self {
            _marker: core::marker::PhantomData,
            _marker2: core::marker::PhantomData,
            pipeline,
            cursor,
            executor,
        }
    }

    /// Advances the derivation pipeline to the target block number.
    ///
    /// ## Takes
    /// - `cfg`: The rollup configuration.
    /// - `target`: The target block number.
    ///
    /// ## Returns
    /// - `Ok((number, output_root))` - A tuple containing the number of the produced block and the
    ///   output root.
    /// - `Err(e)` - An error if the block could not be produced.
    pub async fn advance_to_target(
        &mut self,
        cfg: &RollupConfig,
        mut target: u64,
    ) -> DriverResult<(u64, B256), E::Error> {
        loop {
            // Check if we have reached the target block number.
            if self.cursor.l2_safe_head().block_info.number >= target {
                info!(target: "client", "Derivation complete, reached L2 safe head.");
                return Ok((
                    self.cursor.l2_safe_head().block_info.number,
                    *self.cursor.l2_safe_head_output_root(),
                ));
            }

            let OpAttributesWithParent { mut attributes, .. } = match self
                .pipeline
                .produce_payload(*self.cursor.l2_safe_head())
                .await
            {
                Ok(attrs) => attrs,
                Err(PipelineErrorKind::Critical(PipelineError::EndOfSource)) => {
                    warn!(target: "client", "Exhausted data source; Halting derivation and using current safe head.");

                    // Adjust the target block number to the current safe head, as no more blocks
                    // can be produced.
                    target = self.cursor.l2_safe_head().block_info.number;
                    continue;
                }
                Err(e) => {
                    error!(target: "client", "Failed to produce payload: {:?}", e);
                    return Err(DriverError::Pipeline(e));
                }
            };

            let mut executor =
                self.executor.new_executor(self.cursor.l2_safe_head_header().clone());
            let header = match executor.execute_payload(attributes.clone()) {
                Ok(header) => header,
                Err(e) => {
                    error!(target: "client", "Failed to execute L2 block: {}", e);

                    if cfg.is_holocene_active(attributes.payload_attributes.timestamp) {
                        // Retry with a deposit-only block.
                        warn!(target: "client", "Flushing current channel and retrying deposit only block");

                        // Flush the current batch and channel - if a block was replaced with a
                        // deposit-only block due to execution failure, the
                        // batch and channel it is contained in is forwards
                        // invalidated.
                        self.pipeline.signal(Signal::FlushChannel).await?;

                        // Strip out all transactions that are not deposits.
                        attributes.transactions = attributes.transactions.map(|txs| {
                            txs.into_iter()
                                .filter(|tx| (!tx.is_empty() && tx[0] == OpTxType::Deposit as u8))
                                .collect::<Vec<_>>()
                        });

                        // Retry the execution.
                        executor =
                            self.executor.new_executor(self.cursor.l2_safe_head_header().clone());
                        match executor.execute_payload(attributes.clone()) {
                            Ok(header) => header,
                            Err(e) => {
                                error!(
                                    target: "client",
                                    "Critical - Failed to execute deposit-only block: {e}",
                                );
                                return Err(DriverError::Executor(e));
                            }
                        }
                    } else {
                        // Pre-Holocene, discard the block if execution fails.
                        continue;
                    }
                }
            };

            // Construct the block.
            let block = OpBlock {
                header: header.clone(),
                body: BlockBody {
                    transactions: attributes
                        .transactions
                        .unwrap_or_default()
                        .into_iter()
                        .map(|tx| OpTxEnvelope::decode(&mut tx.as_ref()).map_err(DriverError::Rlp))
                        .collect::<DriverResult<Vec<OpTxEnvelope>, E::Error>>()?,
                    ommers: Vec::new(),
                    withdrawals: None,
                },
            };

            // Get the pipeline origin and update the cursor.
            let origin = self.pipeline.origin().ok_or(PipelineError::MissingOrigin.crit())?;
            let l2_info = L2BlockInfo::from_block_and_genesis(
                &block,
                &self.pipeline.rollup_config().genesis,
            )?;
            let cursor = TipCursor::new(
                l2_info,
                header.clone().seal_slow(),
                executor.compute_output_root().map_err(DriverError::Executor)?,
            );
            self.cursor.advance(origin, cursor);
        }
    }
}