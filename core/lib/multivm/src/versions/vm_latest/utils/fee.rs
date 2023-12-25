//! Utility functions for vm
use micro_system_constants::MAX_GAS_PER_PUBDATA_BYTE;
use micro_utils::ceil_div;

use crate::vm_latest::old_vm::utils::eth_price_per_pubdata_byte;

/// Calculates the amount of gas required to publish one byte of pubdata
pub fn base_fee_to_gas_per_pubdata(l1_gas_price: u64, base_fee: u64) -> u64 {
    let eth_price_per_pubdata_byte = eth_price_per_pubdata_byte(l1_gas_price);

    ceil_div(eth_price_per_pubdata_byte, base_fee)
}

/// Calculates the base fee and gas per pubdata for the given L1 gas price.
pub fn derive_base_fee_and_gas_per_pubdata(l1_gas_price: u64, fair_gas_price: u64) -> (u64, u64) {
    let eth_price_per_pubdata_byte = eth_price_per_pubdata_byte(l1_gas_price);

    // The baseFee is set in such a way that it is always possible for a transaction to
    // publish enough public data while compensating us for it.
    let base_fee = std::cmp::max(
        fair_gas_price,
        ceil_div(eth_price_per_pubdata_byte, MAX_GAS_PER_PUBDATA_BYTE),
    );

    (
        base_fee,
        base_fee_to_gas_per_pubdata(l1_gas_price, base_fee),
    )
}

/// Derives the price for L2 gas in ETH to be paid.
pub fn derive_l2_gas_price(l1_gas_price: u64, fair_l2_gas_price: u64) -> u64 {
    let eth_price_per_pubdata_byte = eth_price_per_pubdata_byte(l1_gas_price);

    // The baseFee is set in such a way that it is always possible for a transaction to
    // publish enough public data while compensating us for it.
    let gas_price = std::cmp::max(
        fair_l2_gas_price,
        ceil_div(eth_price_per_pubdata_byte, MAX_GAS_PER_PUBDATA_BYTE),
    );

    gas_price
}
