//! The network where the micro resides.
//!

// Built-in uses
use std::{fmt, str::FromStr};

// External uses
use serde::{Deserialize, Serialize};

// Workspace uses
use crate::L1ChainId;

// Local uses

/// Network to be used for a micro client.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Network {
    /// Filecoin Mainnet.
    Mainnet,
    /// Filecoin Calibration testnet.
    Calibration,
    /// Self-hosted Filecoin network.
    Localhost,
    /// Unknown network type.
    Unknown,
    /// Test network for testkit purposes
    Test,
}

impl FromStr for Network {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Ok(match string {
            "mainnet" => Self::Mainnet,
            "calibration" => Self::Calibration,
            "localhost" => Self::Localhost,
            "test" => Self::Test,
            another => return Err(another.to_owned()),
        })
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mainnet => write!(f, "mainnet"),
            Self::Calibration => write!(f, "calibration"),
            Self::Localhost => write!(f, "localhost"),
            Self::Unknown => write!(f, "unknown"),
            Self::Test => write!(f, "test"),
        }
    }
}

impl Network {
    /// Returns the network chain ID on the Ethereum side.
    pub fn from_chain_id(chain_id: L1ChainId) -> Self {
        match *chain_id {
            314 => Self::Mainnet,
            314159 => Self::Calibration,
            31415926 => Self::Localhost,
            _ => Self::Unknown,
        }
    }

    /// Returns the network chain ID on the Ethereum side.
    pub fn chain_id(self) -> L1ChainId {
        match self {
            Self::Mainnet => L1ChainId(314),
            Self::Calibration => L1ChainId(314159),
            Self::Localhost => L1ChainId(31415926),
            Self::Unknown => panic!("Unknown chain ID"),
            Self::Test => panic!("Test chain ID"),
        }
    }
}
