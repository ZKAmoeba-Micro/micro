use micro_basic_types::{H256, U256};
use micro_utils::h256_to_u256;
use once_cell::sync::Lazy;

///
/// Well known-slots (e.g. proxy addresses in popular EIPs).
///

const ERC1967_ROLLBACK_SLOT: H256 = H256([
    0x49, 0x10, 0xfd, 0xfa, 0x16, 0xfe, 0xd3, 0x26, 0x0e, 0xd0, 0xe7, 0x14, 0x7f, 0x7c, 0xc6, 0xda,
    0x11, 0xa6, 0x02, 0x08, 0xb5, 0xb9, 0x40, 0x6d, 0x12, 0xa6, 0x35, 0x61, 0x4f, 0xfd, 0x91, 0x43,
]);

const ERC1967_IMPLEMENTATION_SLOT: H256 = H256([
    0x36, 0x08, 0x94, 0xa1, 0x3b, 0xa1, 0xa3, 0x21, 0x06, 0x67, 0xc8, 0x28, 0x49, 0x2d, 0xb9, 0x8d,
    0xca, 0x3e, 0x20, 0x76, 0xcc, 0x37, 0x35, 0xa9, 0x20, 0xa3, 0xca, 0x50, 0x5d, 0x38, 0x2b, 0xbc,
]);

const ERC1967_ADMIN_SLOT: H256 = H256([
    0xb5, 0x31, 0x27, 0x68, 0x4a, 0x56, 0x8b, 0x31, 0x73, 0xae, 0x13, 0xb9, 0xf8, 0xa6, 0x01, 0x6e,
    0x24, 0x3e, 0x63, 0xb6, 0xe8, 0xee, 0x11, 0x78, 0xd6, 0xa7, 0x17, 0x85, 0x0b, 0x5d, 0x61, 0x03,
]);

const ERC1967_BEACON_SLOT: H256 = H256([
    0xa3, 0xf0, 0xad, 0x74, 0xe5, 0x42, 0x3a, 0xeb, 0xfd, 0x80, 0xd3, 0xef, 0x43, 0x46, 0x57, 0x83,
    0x35, 0xa9, 0xa7, 0x2a, 0xea, 0xee, 0x59, 0xff, 0x6c, 0xb3, 0x58, 0x2b, 0x35, 0x13, 0x3d, 0x50,
]);

const INITIALIZER_INITIALING_SLOT: H256 = H256([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
]);

pub static TRUSTED_TOKEN_SLOTS: Lazy<Vec<U256>> = Lazy::new(|| {
    vec![
        ERC1967_ROLLBACK_SLOT,
        ERC1967_IMPLEMENTATION_SLOT,
        ERC1967_ADMIN_SLOT,
        INITIALIZER_INITIALING_SLOT,
    ]
    .into_iter()
    .map(h256_to_u256)
    .collect()
});

// These slots contain addresses that should themselves be trusted.
pub static TRUSTED_ADDRESS_SLOTS: Lazy<Vec<U256>> = Lazy::new(|| {
    vec![ERC1967_BEACON_SLOT]
        .into_iter()
        .map(h256_to_u256)
        .collect()
});