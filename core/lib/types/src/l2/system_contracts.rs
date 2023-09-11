use std::convert::TryFrom;

use micro_basic_types::{ethabi::Token, Address, U256};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Normol,
    FrozenNode,
    Node,
    WaitingNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_address: Address,
    pub deposit_amount: U256,
    pub node_role: Role,
}

pub struct NodeInfoList {
    pub list: Vec<NodeInfo>,
}

impl TryFrom<Vec<Token>> for NodeInfoList {
    type Error = crate::ethabi::Error;

    fn try_from(mut tokens: Vec<Token>) -> Result<Self, Self::Error> {
        let token = tokens.remove(0);
        let tuples = token.into_array().unwrap();

        let mut result = vec![];
        for tuple in tuples {
            match tuple {
                Token::Tuple(mut t) => {
                    let node_address = t.remove(0).into_address().unwrap();
                    let deposit_amount = t.remove(0).into_uint().unwrap();
                    let node_role_u256 = t.remove(0).into_uint().unwrap().as_u32();
                    let node_role = match node_role_u256 {
                        0 => Role::Normol,
                        1 => Role::FrozenNode,
                        2 => Role::Node,
                        3 => Role::WaitingNode,
                        _ => return Err(Self::Error::InvalidData),
                    };

                    result.push(NodeInfo {
                        node_address,
                        deposit_amount,
                        node_role,
                    });
                }
                _ => return Err(Self::Error::InvalidData),
            }
        }

        Ok(NodeInfoList { list: result })
    }
}
