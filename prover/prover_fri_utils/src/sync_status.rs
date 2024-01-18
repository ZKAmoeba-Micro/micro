use anyhow::{Error, Ok};
use micro_dal::ConnectionPool;
use micro_types::U64;
use micro_web3_decl::{
    jsonrpsee::http_client::HttpClientBuilder,
    namespaces::{EthNamespaceClient, ZksNamespaceClient},
};

pub async fn get_sync_status(pool: ConnectionPool, rpc_url: &str) -> Result<bool, Error> {
    let mut connection = pool.access_storage().await.unwrap();
    let client = HttpClientBuilder::default()
        .build(rpc_url)
        .expect("faile to build rpc client");

    let local_batch_number = connection
        .blocks_web3_dal()
        .get_sealed_l1_batch_number()
        .await
        .map(|n| U64::from(n.0))
        .map_err(|e| {
            tracing::error!("failed to get local block number ");
            e
        })?;

    let latest_batch_number = client.get_l1_batch_number().await.map_err(|e| {
        tracing::error!("failed to get latest block number ");
        e
    })?;

    tracing::info!(
        " local_batch_number {:?}  latest_batch_number {:?}",
        local_batch_number,
        latest_batch_number
    );

    Ok(local_batch_number.eq(&latest_batch_number))
}
