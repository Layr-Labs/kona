use alloy_primitives::Bytes;
use alloy_rlp::{BufMut, BytesMut};
use op_alloy_protocol::DERIVATION_VERSION_0;

// TODO: upstream to op_alloy_protocol
pub(crate) const DERIVATION_VERSION_1: u8 = 1;

#[derive(Debug, PartialEq)]
/// Submission represents an op-batcher tx's calldata.
/// See https://specs.optimism.io/experimental/alt-da.html#input-commitment-submission
pub(crate) enum BatcherSubmission {
    Frames(Bytes),
    // For now a batcher tx can only submit a single commitment.
    Commitment(AltDACommitment),
}

#[derive(Debug, PartialEq)]
pub enum AltDACommitment {
    Keccak(Bytes),
    EigenDAV1(Bytes),
    EigenDAV2(Bytes),
    Avail(Bytes),
    Celestia(Bytes),
}

impl AltDACommitment {
    /// Returns the payload of the commitment, which is the part of the commitment
    /// that is specific to the altda layer.
    pub fn payload(&self) -> &Bytes {
        match self {
            AltDACommitment::Keccak(bytes) => bytes,
            AltDACommitment::EigenDAV1(bytes) => bytes,
            AltDACommitment::EigenDAV2(bytes) => bytes,
            AltDACommitment::Avail(bytes) => bytes,
            AltDACommitment::Celestia(bytes) => bytes,
        }
    }

    /// Converts the commitment to its byte representation, following the format specified in
    /// https://specs.optimism.io/experimental/alt-da.html#input-commitment-submission
    pub fn to_commitment(&self) -> Bytes {
        let mut commitment = BytesMut::new();
        commitment.put_u8(DERIVATION_VERSION_1);
        match self {
            AltDACommitment::Keccak(bytes) => {
                commitment.put_u8(0);
                commitment.extend_from_slice(bytes);
            }
            AltDACommitment::EigenDAV1(bytes) => {
                commitment.put_u8(1); // generic commitment
                commitment.put_u8(0); // eigenda
                commitment.put_u8(0); // v1
                commitment.extend_from_slice(bytes);
            }
            AltDACommitment::EigenDAV2(bytes) => {
                commitment.put_u8(1); // generic commitment
                commitment.put_u8(0); // eigenda
                commitment.put_u8(1); // v2
                commitment.extend_from_slice(bytes);
            }
            AltDACommitment::Avail(bytes) => {
                commitment.put_u8(1); // generic commitment
                commitment.put_u8(0x0a);
                commitment.extend_from_slice(bytes);
            }
            AltDACommitment::Celestia(bytes) => {
                commitment.put_u8(1); // generic commitment
                commitment.put_u8(0x0c);
                commitment.extend_from_slice(bytes);
            }
        }
        commitment.freeze().into()
    }
}

impl BatcherSubmission {
    /// Parses the submission from the given bytes, following the format specified in
    /// https://specs.optimism.io/experimental/alt-da.html#input-commitment-submission
    pub(crate) fn parse(bytes: Bytes) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }
        match bytes[0] {
            DERIVATION_VERSION_0 => Some(BatcherSubmission::Frames(bytes.slice(1..))),
            DERIVATION_VERSION_1 if bytes.len() > 1 => match bytes[1] {
                0 if bytes.len() == 2 + 32 => {
                    let commitment = AltDACommitment::Keccak(bytes.slice(2..));
                    Some(BatcherSubmission::Commitment(commitment))
                }
                1 if bytes.len() > 2 => {
                    let altda_commitment = match bytes[2] {
                        // See https://github.com/ethereum-optimism/specs/discussions/135
                        0 if bytes.len() > 3 => match bytes[3] {
                            0 => AltDACommitment::EigenDAV1(bytes.slice(4..)),
                            1 => AltDACommitment::EigenDAV2(bytes.slice(4..)),
                            _ => return None,
                        },
                        0x0a => AltDACommitment::Avail(bytes.slice(3..)),
                        0x0c => AltDACommitment::Celestia(bytes.slice(3..)),
                        _ => return None,
                    };
                    Some(BatcherSubmission::Commitment(altda_commitment))
                }
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_commitment() {
        let keccak_commitment = AltDACommitment::Keccak(Bytes::from_static(b"keccak_commitment"));
        let eigenda_v1_commitment =
            AltDACommitment::EigenDAV1(Bytes::from_static(b"eigenda_commitment"));
        let avail_commitment = AltDACommitment::Avail(Bytes::from_static(b"avail_commitment"));
        let celestia_commitment =
            AltDACommitment::Celestia(Bytes::from_static(b"celestia_commitment"));

        assert_eq!(keccak_commitment.to_commitment(), {
            let mut commitment = BytesMut::new();
            commitment.put_u8(DERIVATION_VERSION_1);
            commitment.put_u8(0);
            commitment.extend_from_slice(b"keccak_commitment");
            commitment.freeze()
        });

        assert_eq!(eigenda_v1_commitment.to_commitment(), {
            let mut commitment = BytesMut::new();
            commitment.put_u8(DERIVATION_VERSION_1);
            commitment.put_u8(1);
            commitment.put_u8(0);
            commitment.put_u8(0);
            commitment.extend_from_slice(b"eigenda_commitment");
            commitment.freeze()
        });

        assert_eq!(avail_commitment.to_commitment(), {
            let mut commitment = BytesMut::new();
            commitment.put_u8(DERIVATION_VERSION_1);
            commitment.put_u8(1);
            commitment.put_u8(0x0a);
            commitment.extend_from_slice(b"avail_commitment");
            commitment.freeze()
        });

        assert_eq!(celestia_commitment.to_commitment(), {
            let mut commitment = BytesMut::new();
            commitment.put_u8(DERIVATION_VERSION_1);
            commitment.put_u8(1);
            commitment.put_u8(0x0c);
            commitment.extend_from_slice(b"celestia_commitment");
            commitment.freeze()
        });
    }

    #[test]
    fn test_parse() {
        let frames = Bytes::from_static(b"\x00frames");
        let keccak_commitment = Bytes::from_static(b"\x01\x0012345678901234567890123456789012");
        let fake_keccak_commitment = Bytes::from_static(b"\x01\x00not_a_keccak_commitment");
        let eigenda_v1_commitment = Bytes::from_static(b"\x01\x01\x00\x00eigenda_commitment");
        let avail_commitment = Bytes::from_static(b"\x01\x01\x0aavail_commitment");
        let celestia_commitment = Bytes::from_static(b"\x01\x01\x0ccelestia_commitment");

        assert_eq!(
            BatcherSubmission::parse(frames.clone()),
            Some(BatcherSubmission::Frames(frames.slice(1..)))
        );
        assert_eq!(BatcherSubmission::parse(fake_keccak_commitment.clone()), None);
        assert_eq!(
            BatcherSubmission::parse(keccak_commitment.clone()),
            Some(BatcherSubmission::Commitment(AltDACommitment::Keccak(
                keccak_commitment.slice(2..)
            )))
        );
        assert_eq!(
            BatcherSubmission::parse(eigenda_v1_commitment.clone()),
            Some(BatcherSubmission::Commitment(AltDACommitment::EigenDAV1(
                eigenda_v1_commitment.slice(4..)
            )))
        );
        assert_eq!(
            BatcherSubmission::parse(avail_commitment.clone()),
            Some(BatcherSubmission::Commitment(AltDACommitment::Avail(
                avail_commitment.slice(3..)
            )))
        );
        assert_eq!(
            BatcherSubmission::parse(celestia_commitment.clone()),
            Some(BatcherSubmission::Commitment(AltDACommitment::Celestia(
                celestia_commitment.slice(3..)
            )))
        );
    }
}
