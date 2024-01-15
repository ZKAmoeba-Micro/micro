use std::sync::Arc;

use axum::{extract::State, Json};
use bigdecimal::{BigDecimal, FromPrimitive, Zero};
use micro_contracts::{sys_deposit_contract, sys_white_list_contract};
use micro_system_constants::{DEPOSIT_ADDRESS, WHITE_LIST_ADDRESS};
use micro_types::{
    api::{BlockIdVariant, BlockNumber},
    ethabi::{Address, Token},
    transaction_request::CallRequest,
    tx::primitives::PackedEthSignature,
};
use micro_web3_decl::namespaces::EthNamespaceClient;
use serde::{Deserialize, Serialize};

use crate::{dashboard::Dashboard, error::DashboardError};

#[derive(Debug, Serialize, Deserialize)]
pub enum Status {
    UnDeposit,
    Normal,
    Frozen,
    Applying,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub in_white_list: bool,
    pub amount: String,
    pub status: Status,
}

pub async fn get(State(state): State<Arc<Dashboard>>) -> Result<Json<Response>, DashboardError> {
    //get private key and  wallet address from config
    let operator_private_key = state
        .config
        .prover_private_key()
        .expect("Operator private key is required for signing client");
    let wallet_address = PackedEthSignature::address_from_private_key(&operator_private_key)
        .map_err(|_| {
            DashboardError::RpcError("Failed to get address from private key".to_string())
        })?;

    let white_list_contract = sys_white_list_contract();
    let deposit_contract = sys_deposit_contract();

    let white_list_contract_function = white_list_contract
        .function("whiteList")
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let white_list_data = white_list_contract_function
        .encode_input(&[Token::Address(wallet_address)])
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let white_list_req = CallRequest {
        to: Some(WHITE_LIST_ADDRESS),
        data: Some(white_list_data.into()),
        ..Default::default()
    };
    let block = Some(BlockIdVariant::BlockNumber(BlockNumber::Latest));

    let white_list_resp = state
        .client
        .call(white_list_req, block)
        .await
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let mut white_list_result = white_list_contract_function
        .decode_output(&white_list_resp.0)
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let in_white_list = white_list_result.remove(0).into_bool().unwrap_or(false);

    //get mainToken from deposit contract
    let main_token_function = deposit_contract
        .function("mainToken")
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let main_token_call_data = main_token_function
        .encode_input(&[])
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let main_token_req = CallRequest {
        to: Some(DEPOSIT_ADDRESS),
        data: Some(main_token_call_data.into()),
        ..Default::default()
    };
    let main_token_resp = state
        .client
        .call(main_token_req, block)
        .await
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let mut main_token_result = main_token_function
        .decode_output(&main_token_resp.0)
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let main_token = main_token_result
        .remove(0)
        .into_address()
        .unwrap_or(Address::zero());

    // get main token deposit info
    let dposit_info_function = deposit_contract
        .function("getProverTokenDepositInfo")
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;
    let deposit_info_call_data = dposit_info_function
        .encode_input(&[
            Token::Address(wallet_address), //prover address
            Token::Address(main_token),     //token address
        ])
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let deposit_info_req = CallRequest {
        to: Some(DEPOSIT_ADDRESS),
        data: Some(deposit_info_call_data.into()),
        ..Default::default()
    };
    let deposit_info_resp = state
        .client
        .call(deposit_info_req, block)
        .await
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let mut deposit_info_result = dposit_info_function
        .decode_output(&deposit_info_resp.0)
        .map_err(|e| DashboardError::RpcError(e.to_string()))?;

    let status = deposit_info_result
        .remove(0)
        .into_int()
        .map_or(Status::UnDeposit, |x| match x.as_u32() {
            1 => Status::Normal,
            2 => Status::Frozen,
            3 => Status::Applying,
            _ => Status::UnDeposit,
        });

    let _apply_time = deposit_info_result.remove(0).into_uint();

    let amount = deposit_info_result
        .remove(0)
        .into_uint()
        .map_or(BigDecimal::zero().to_string(), |x| {
            BigDecimal::from_u128(x.as_u128()).unwrap().to_string()
        });
    let _deposit_time = deposit_info_result.remove(0).into_uint();

    Ok(Json(Response {
        in_white_list,
        amount,
        status,
    }))
}
