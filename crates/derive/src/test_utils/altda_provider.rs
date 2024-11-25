//! An implementation of the [AltDAProvider] trait for tests.

use crate::errors::{PipelineError, PipelineErrorKind};
use crate::prelude::AltDACommitment;
use crate::traits::AltDAProvider;
use alloc::boxed::Box;
use alloy_primitives::map::HashMap;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use core::fmt::Debug;
use thiserror::Error;

/// Mock data availability provider
#[derive(Debug, Default, Clone)]
pub struct TestAltDAProvider {
    blobs: HashMap<Bytes, Bytes>,
}

impl TestAltDAProvider {
    pub fn new() -> Self {
        Self { blobs: HashMap::new() }
    }

    pub fn insert_blob(&mut self, commitment: Bytes, blob: Bytes) {
        self.blobs.insert(commitment, blob);
    }
}

/// An error for the [TestChainProvider] and [TestL2ChainProvider].
#[derive(Error, Debug)]
pub enum TestAltDAProviderError {
    /// The blob was not found.
    #[error("Blob not found")]
    BlobNotFound,
}

impl From<TestAltDAProviderError> for PipelineErrorKind {
    fn from(val: TestAltDAProviderError) -> Self {
        PipelineError::Provider(val.to_string()).temp()
    }
}

#[async_trait]
impl AltDAProvider for TestAltDAProvider {
    type Error = TestAltDAProviderError;

    async fn get_blob(&self, commitment: AltDACommitment) -> Result<Bytes, Self::Error> {
        // We extract the bytes out of the altda commitment, regardless of the altda layer.
        // Basically every altda layer is mixed into a single HashMap in this test provider
        self.blobs.get(commitment.payload()).cloned().ok_or(TestAltDAProviderError::BlobNotFound)
    }
}
