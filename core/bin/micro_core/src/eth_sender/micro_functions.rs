use micro_contracts::micro_contract;
use micro_types::ethabi::Function;

#[derive(Debug)]
pub(super) struct MicroFunctions {
    pub(super) commit_blocks: Function,
    pub(super) prove_blocks: Function,
    pub(super) execute_blocks: Function,
}

pub(super) fn get_micro_functions() -> MicroFunctions {
    let micro_contract = micro_contract();

    let commit_blocks = micro_contract
        .functions
        .get("commitBlocks")
        .cloned()
        .expect("commitBlocks function not found")
        .pop()
        .expect("commitBlocks function entry not found");

    let prove_blocks = micro_contract
        .functions
        .get("proveBlocks")
        .cloned()
        .expect("proveBlocks function not found")
        .pop()
        .expect("proveBlocks function entry not found");

    let execute_blocks = micro_contract
        .functions
        .get("executeBlocks")
        .cloned()
        .expect("executeBlocks function not found")
        .pop()
        .expect("executeBlocks function entry not found");

    MicroFunctions {
        commit_blocks,
        prove_blocks,
        execute_blocks,
    }
}
