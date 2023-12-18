use micro_contracts::{
    micro_contract, multicall_contract, verifier_contract, PRE_BOOJUM_COMMIT_FUNCTION,
    PRE_BOOJUM_EXECUTE_FUNCTION, PRE_BOOJUM_GET_VK_FUNCTION, PRE_BOOJUM_PROVE_FUNCTION,
};
use micro_types::ethabi::{Contract, Function};

#[derive(Debug)]
pub(super) struct MicroFunctions {
    pub(super) pre_boojum_commit: Function,
    pub(super) post_boojum_commit: Option<Function>,
    pub(super) pre_boojum_prove: Function,
    pub(super) post_boojum_prove: Option<Function>,
    pub(super) pre_boojum_execute: Function,
    pub(super) post_boojum_execute: Option<Function>,
    pub(super) get_l2_bootloader_bytecode_hash: Function,
    pub(super) get_l2_default_account_bytecode_hash: Function,
    pub(super) get_verifier: Function,
    pub(super) get_verifier_params: Function,
    pub(super) get_protocol_version: Function,

    pub(super) verifier_contract: Contract,
    pub(super) get_verification_key: Function,
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

        let pre_boojum_commit = PRE_BOOJUM_COMMIT_FUNCTION.clone();
        let post_boojum_commit = get_optional_function(&micro_contract, "commitBatches");
        let pre_boojum_prove = PRE_BOOJUM_PROVE_FUNCTION.clone();
        let post_boojum_prove = get_optional_function(&micro_contract, "proveBatches");
        let pre_boojum_execute = PRE_BOOJUM_EXECUTE_FUNCTION.clone();
        let post_boojum_execute = get_optional_function(&micro_contract, "executeBatches");
        let get_l2_bootloader_bytecode_hash =
            get_function(&micro_contract, "getL2BootloaderBytecodeHash");
        let get_l2_default_account_bytecode_hash =
            get_function(&micro_contract, "getL2DefaultAccountBytecodeHash");
        let get_verifier = get_function(&micro_contract, "getVerifier");
        let get_verifier_params = get_function(&micro_contract, "getVerifierParams");
        let get_protocol_version = get_function(&micro_contract, "getProtocolVersion");
        let get_verification_key = PRE_BOOJUM_GET_VK_FUNCTION.clone();
        let aggregate3 = get_function(&multicall_contract, "aggregate3");
        let verification_key_hash =
            get_optional_function(&verifier_contract, "verificationKeyHash");

        MicroFunctions {
            pre_boojum_commit,
            post_boojum_commit,
            pre_boojum_prove,
            post_boojum_prove,
            pre_boojum_execute,
            post_boojum_execute,
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
