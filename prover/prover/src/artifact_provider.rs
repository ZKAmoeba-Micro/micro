use prover_service::ArtifactProvider;
use std::io::Read;
use anyhow::Context as _;
use zkevm_test_harness::abstract_micro_circuit::concrete_circuits::MicroVerificationKey;
use zkevm_test_harness::pairing::bn256::Bn256;
use micro_setup_key_server::get_setup_for_circuit_type;
use micro_verification_key_server::get_vk_for_circuit_type;

#[derive(Debug)]
pub struct ProverArtifactProvider;

impl ArtifactProvider for ProverArtifactProvider {
    type ArtifactError = anyhow::Error;

    fn get_setup(&self, circuit_id: u8) -> Result<Box<dyn Read>, Self::ArtifactError> {
        get_setup_for_circuit_type(circuit_id).context("get_setup_for_circuit_type()")
    }

    fn get_vk(&self, circuit_id: u8) -> Result<MicroVerificationKey<Bn256>, Self::ArtifactError> {
        let vk = get_vk_for_circuit_type(circuit_id);
        Ok(MicroVerificationKey::from_verification_key_and_numeric_type(circuit_id, vk))
    }
}