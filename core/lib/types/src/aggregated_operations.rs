use std::{fmt, ops, str::FromStr};

use codegen::serialize_proof;
use micro_basic_types::{
    ethabi::{Token, Uint},
    Address, L1BatchNumber, H256, U256,
};
use serde::{Deserialize, Serialize};
use zkevm_test_harness::{
    abstract_micro_circuit::concrete_circuits::MicroCircuit,
    bellman::{bn256::Bn256, plonk::better_better_cs::proof::Proof},
    witness::oracle::VmWitnessOracle,
};

use crate::{commitment::L1BatchWithMetadata, PackedEthSignature, ProtocolVersionId};

fn l1_batch_range_from_batches(
    batches: &[L1BatchWithMetadata],
) -> ops::RangeInclusive<L1BatchNumber> {
    let start = batches
        .first()
        .map(|l1_batch| l1_batch.header.number)
        .unwrap_or_default();
    let end = batches
        .last()
        .map(|l1_batch| l1_batch.header.number)
        .unwrap_or_default();
    start..=end
}

#[derive(Debug, Clone)]
pub struct L1BatchCommitOperation {
    pub last_committed_l1_batch: L1BatchWithMetadata,
    pub l1_batches: Vec<L1BatchWithMetadata>,
}

impl L1BatchCommitOperation {
    pub fn get_eth_tx_args(&self) -> Vec<Token> {
        let stored_batch_info = self.last_committed_l1_batch.l1_header_data();
        let l1_batches_to_commit = self
            .l1_batches
            .iter()
            .map(L1BatchWithMetadata::l1_commit_data)
            .collect();

        vec![stored_batch_info, Token::Array(l1_batches_to_commit)]
    }

    pub fn l1_batch_range(&self) -> ops::RangeInclusive<L1BatchNumber> {
        l1_batch_range_from_batches(&self.l1_batches)
    }
}

#[derive(Debug, Clone)]
pub struct L1BatchCreateProofOperation {
    pub l1_batches: Vec<L1BatchWithMetadata>,
    pub proofs_to_pad: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct L1BatchProofForL1 {
    pub aggregation_result_coords: [[u8; 32]; 4],
    pub scheduler_proof: Proof<Bn256, MicroCircuit<Bn256, VmWitnessOracle<Bn256>>>,
    pub signature: PackedEthSignature,
    pub time_taken: u64,
}

impl fmt::Debug for L1BatchProofForL1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("L1BatchProofForL1")
            .field("aggregation_result_coords", &self.aggregation_result_coords)
            .finish_non_exhaustive()
    }
}

impl L1BatchProofForL1 {
    pub fn sign(&mut self, l1_batch_number: L1BatchNumber, private_key: &H256) {
        self.signature = PackedEthSignature::sign(private_key, &self.encode(l1_batch_number))
            .expect("sign l1 batch proof failed");
    }

    pub fn signature_recover_signer(
        &self,
        l1_batch_number: L1BatchNumber,
    ) -> Result<Address, parity_crypto::publickey::Error> {
        if self.signature == PackedEthSignature::default() {
            return Ok(Address::default());
        }
        let signed_bytes =
            PackedEthSignature::message_to_signed_bytes(&self.encode(l1_batch_number));
        self.signature.signature_recover_signer(&signed_bytes)
    }

    pub fn proof_input(&self, is_pre_boojum: bool) -> Token {
        let aggregation_result_coords = if is_pre_boojum {
            Token::Array(
                self.aggregation_result_coords
                    .iter()
                    .map(|bytes| Token::Uint(U256::from_big_endian(bytes)))
                    .collect(),
            )
        } else {
            Token::Array(Vec::new())
        };

        let (_inputs, proof) = serialize_proof(&self.scheduler_proof);
        Token::Tuple(vec![
            aggregation_result_coords,
            Token::Array(proof.into_iter().map(Token::Uint).collect()),
        ])
    }

    fn encode(&self, l1_batch_number: L1BatchNumber) -> Vec<u8> {
        crate::ethabi::encode(&vec![
            self.proof_input(true),
            Token::Uint(l1_batch_number.0.into()),
        ])
    }
}

#[derive(Debug, Clone)]
pub struct L1BatchProofOperation {
    pub prev_l1_batch: L1BatchWithMetadata,
    pub l1_batches: Vec<L1BatchWithMetadata>,
    pub proofs: Vec<L1BatchProofForL1>,
    pub should_verify: bool,
}

impl L1BatchProofOperation {
    pub fn get_eth_tx_args(&self) -> Vec<Token> {
        let prev_l1_batch = self.prev_l1_batch.l1_header_data();
        let batches_arg = self
            .l1_batches
            .iter()
            .map(L1BatchWithMetadata::l1_header_data)
            .collect();
        let batches_arg = Token::Array(batches_arg);

        if self.should_verify {
            // currently we only support submitting a single proof
            assert_eq!(self.proofs.len(), 1);
            assert_eq!(self.l1_batches.len(), 1);

            let l1_batch_number = self.l1_batches.first().unwrap().header.number;

            let l1_batch_proof_for_l1 = self.proofs.first().unwrap();

            let is_pre_boojum = self.l1_batches[0]
                .header
                .protocol_version
                .unwrap()
                .is_pre_boojum();

            let proof_input = l1_batch_proof_for_l1.proof_input(is_pre_boojum);
            let prover_info = Token::Tuple(vec![
                Token::Address(
                    l1_batch_proof_for_l1
                        .signature_recover_signer(l1_batch_number)
                        .unwrap(),
                ),
                Token::Uint(Uint::from(l1_batch_proof_for_l1.time_taken)),
            ]);

            vec![prev_l1_batch, batches_arg, proof_input, prover_info]
        } else {
            vec![
                prev_l1_batch,
                batches_arg,
                Token::Tuple(vec![Token::Array(vec![]), Token::Array(vec![])]),
                Token::Tuple(vec![
                    Token::Address(Address::default()),
                    Token::Uint(Uint::from(0)),
                ]),
            ]
        }
    }

    pub fn l1_batch_range(&self) -> ops::RangeInclusive<L1BatchNumber> {
        l1_batch_range_from_batches(&self.l1_batches)
    }
}

#[derive(Debug, Clone)]
pub struct L1BatchExecuteOperation {
    pub l1_batches: Vec<L1BatchWithMetadata>,
}

impl L1BatchExecuteOperation {
    pub fn get_eth_tx_args(&self) -> Vec<Token> {
        vec![Token::Array(
            self.l1_batches
                .iter()
                .map(L1BatchWithMetadata::l1_header_data)
                .collect(),
        )]
    }

    pub fn l1_batch_range(&self) -> ops::RangeInclusive<L1BatchNumber> {
        l1_batch_range_from_batches(&self.l1_batches)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggregatedActionType {
    Commit,
    PublishProofOnchain,
    Execute,
}

impl AggregatedActionType {
    pub fn as_str(self) -> &'static str {
        // "Blocks" suffixes are there for legacy reasons
        match self {
            Self::Commit => "CommitBlocks",
            Self::PublishProofOnchain => "PublishProofBlocksOnchain",
            Self::Execute => "ExecuteBlocks",
        }
    }
}

impl fmt::Display for AggregatedActionType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for AggregatedActionType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CommitBlocks" => Ok(Self::Commit),
            "PublishProofBlocksOnchain" => Ok(Self::PublishProofOnchain),
            "ExecuteBlocks" => Ok(Self::Execute),
            _ => Err(
                "Incorrect aggregated action type; expected one of `CommitBlocks`, `PublishProofBlocksOnchain`, \
                `ExecuteBlocks`",
            ),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum AggregatedOperation {
    Commit(L1BatchCommitOperation),
    PublishProofOnchain(L1BatchProofOperation),
    Execute(L1BatchExecuteOperation),
}

impl AggregatedOperation {
    pub fn get_action_type(&self) -> AggregatedActionType {
        match self {
            Self::Commit(_) => AggregatedActionType::Commit,
            Self::PublishProofOnchain(_) => AggregatedActionType::PublishProofOnchain,
            Self::Execute(_) => AggregatedActionType::Execute,
        }
    }

    pub fn l1_batch_range(&self) -> ops::RangeInclusive<L1BatchNumber> {
        match self {
            Self::Commit(op) => op.l1_batch_range(),
            Self::PublishProofOnchain(op) => op.l1_batch_range(),
            Self::Execute(op) => op.l1_batch_range(),
        }
    }

    pub fn get_action_caption(&self) -> &'static str {
        match self {
            Self::Commit(_) => "commit",
            Self::PublishProofOnchain(_) => "proof",
            Self::Execute(_) => "execute",
        }
    }

    pub fn protocol_version(&self) -> ProtocolVersionId {
        match self {
            Self::Commit(op) => op.l1_batches[0].header.protocol_version.unwrap(),
            Self::PublishProofOnchain(op) => op.l1_batches[0].header.protocol_version.unwrap(),
            Self::Execute(op) => op.l1_batches[0].header.protocol_version.unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use micro_basic_types::{ethabi::Token, H256, U256};

    use crate::PackedEthSignature;

    #[test]
    fn test_encode_sign() {
        let private_key = H256::from([5; 32]);

        let proof_input = Token::Tuple(vec![
            Token::Array(
                vec![U256::from(1), U256::from(2), U256::from(3)]
                    .into_iter()
                    .map(Token::Uint)
                    .collect(),
            ),
            Token::Array(
                vec![U256::from(4), U256::from(5), U256::from(6)]
                    .into_iter()
                    .map(Token::Uint)
                    .collect(),
            ),
        ]);

        let encoded = crate::ethabi::encode(&vec![proof_input]);

        let res = PackedEthSignature::sign(&private_key, &encoded).unwrap();
        let signature = hex::encode(res.serialize_packed());

        assert_eq!(signature, "a4be061bd5b6251c3dd6873b5181b58112c61c8abda9edb20e8ea195474af4c0043d2c1e9541973b758d809d7c67764a9559de4979f50a902f7a53490d4c5f1f1c");
    }
}
