use micro_contracts::{micro_contract, multicall_contract, verifier_contract};
use micro_types::ethabi::{Contract, Function};

#[derive(Debug)]
pub(super) struct MicroFunctions {
    pub(super) commit_blocks: Function,
    pub(super) prove_blocks: Function,
    pub(super) execute_blocks: Function,
    pub(super) get_l2_bootloader_bytecode_hash: Function,
    pub(super) get_l2_default_account_bytecode_hash: Function,
    pub(super) get_verifier: Function,
    pub(super) get_verifier_params: Function,
    pub(super) get_protocol_version: Function,

    pub(super) verifier_contract: Contract,
    pub(super) get_verification_key: Option<Function>,
    pub(super) verification_key_hash: Option<Function>,

    pub(super) multicall_contract: Contract,
    pub(super) aggregate3: Function,
}

fn get_function(contract: &Contract, name: &str) -> Function {
    contract
        .functions
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("{} function not found", name))
        .pop()
        .unwrap_or_else(|| panic!("{} function entry not found", name))
}

fn get_optional_function(contract: &Contract, name: &str) -> Option<Function> {
    contract
        .functions
        .get(name)
        .cloned()
        .map(|mut functions| functions.pop().unwrap())
}

impl Default for MicroFunctions {
    fn default() -> Self {
        let micro_contract = micro_contract();
        let verifier_contract = verifier_contract();
        let multicall_contract = multicall_contract();

        let commit_blocks = get_function(&micro_contract, "commitBatches");
        let prove_blocks = get_function(&micro_contract, "proveBatches");
        let execute_blocks = get_function(&micro_contract, "executeBatches");
        let get_l2_bootloader_bytecode_hash =
            get_function(&micro_contract, "getL2BootloaderBytecodeHash");
        let get_l2_default_account_bytecode_hash =
            get_function(&micro_contract, "getL2DefaultAccountBytecodeHash");
        let get_verifier = get_function(&micro_contract, "getVerifier");
        let get_verifier_params = get_function(&micro_contract, "getVerifierParams");
        let get_protocol_version = get_function(&micro_contract, "getProtocolVersion");
        let aggregate3 = get_function(&multicall_contract, "aggregate3");
        let get_verification_key =
            get_optional_function(&verifier_contract, "get_verification_key");
        let verification_key_hash =
            get_optional_function(&verifier_contract, "verificationKeyHash");

        assert!(
            get_verification_key.as_ref().xor(verification_key_hash.as_ref()).is_some(),
            "Either get_verification_key or verificationKeyHash must be present in the verifier contract, but not both"
        );

        MicroFunctions {
            commit_blocks,
            prove_blocks,
            execute_blocks,
            get_l2_bootloader_bytecode_hash,
            get_l2_default_account_bytecode_hash,
            get_verifier,
            get_verifier_params,
            get_protocol_version,
            verifier_contract,
            get_verification_key,
            verification_key_hash,
            multicall_contract,
            aggregate3,
        }
    }
}
