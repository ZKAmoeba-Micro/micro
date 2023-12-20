use micro_types::{fee::Fee, Execute, U256};

use crate::{
    interface::{TxExecutionMode, VmInterface},
    vm_latest::{
        constants::{BOOTLOADER_HEAP_PAGE, TX_DESCRIPTION_OFFSET, TX_GAS_LIMIT_OFFSET},
        tests::tester::VmTesterBuilder,
        HistoryDisabled,
    },
    vm_virtual_blocks::constants::{
        TX_MAX_FEE_PER_GAS_OFFSET, TX_MAX_PRIORITY_PER_FEE_OFFSET, TX_TYPE_OFFSET,
    },
};

/// Checks that `TX_GAS_LIMIT_OFFSET`, `TX_MAX_FEE_PER_GAS_OFFSET`, `TX_MAX_PRIORITY_FEE_PER_GAS_OFFSET` constant is correct.
#[test]
fn test_tx_fee_offset() {
    let mut vm = VmTesterBuilder::new(HistoryDisabled)
        .with_empty_in_memory_storage()
        .with_execution_mode(TxExecutionMode::VerifyExecute)
        .with_random_rich_accounts(1)
        .build();

    let gas_limit = 9999.into();
    let max_fee_per_gas = 9998.into();
    let max_priority_fee_per_gas = 9997.into();
    let tx = vm.rich_accounts[0].get_l2_tx_for_execute(
        Execute {
            contract_address: Default::default(),
            calldata: vec![],
            value: Default::default(),
            factory_deps: None,
        },
        Some(Fee {
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            ..Default::default()
        }),
    );

    vm.vm.push_transaction(tx);

    let gas_limit_from_memory = vm
        .vm
        .state
        .memory
        .read_slot(
            BOOTLOADER_HEAP_PAGE as usize,
            TX_DESCRIPTION_OFFSET + TX_GAS_LIMIT_OFFSET,
        )
        .value;
    assert_eq!(gas_limit_from_memory, gas_limit);

    let max_fee_per_gas_from_memory = vm
        .vm
        .state
        .memory
        .read_slot(
            BOOTLOADER_HEAP_PAGE as usize,
            TX_DESCRIPTION_OFFSET + TX_MAX_FEE_PER_GAS_OFFSET,
        )
        .value;
    assert_eq!(max_fee_per_gas_from_memory, max_fee_per_gas);

    let max_priority_fee_per_gas_from_memory = vm
        .vm
        .state
        .memory
        .read_slot(
            BOOTLOADER_HEAP_PAGE as usize,
            TX_DESCRIPTION_OFFSET + TX_MAX_PRIORITY_PER_FEE_OFFSET,
        )
        .value;
    assert_eq!(
        max_priority_fee_per_gas_from_memory,
        max_priority_fee_per_gas
    );
}

/// Checks that `TX_TYPE_OFFSET` constant is correct.
#[test]
fn test_tx_type_offset() {
    let mut vm = VmTesterBuilder::new(HistoryDisabled)
        .with_empty_in_memory_storage()
        .with_execution_mode(TxExecutionMode::VerifyExecute)
        .with_random_rich_accounts(1)
        .build();

    let gas_limit = 9999.into();
    let tx = vm.rich_accounts[0].get_l2_tx_for_execute(
        Execute {
            contract_address: Default::default(),
            calldata: vec![],
            value: Default::default(),
            factory_deps: None,
        },
        Some(Fee {
            gas_limit,
            ..Default::default()
        }),
    );

    vm.vm.push_transaction(tx);

    let tx_type_from_memory = vm
        .vm
        .state
        .memory
        .read_slot(
            BOOTLOADER_HEAP_PAGE as usize,
            TX_DESCRIPTION_OFFSET + TX_TYPE_OFFSET,
        )
        .value;
    assert_eq!(tx_type_from_memory, U256::from(0x71));
}
