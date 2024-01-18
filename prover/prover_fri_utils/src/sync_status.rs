use anyhow::{Error, Ok};
use micro_dal::ConnectionPool;
use micro_types::U64;
use micro_web3_decl::{jsonrpsee::http_client::HttpClientBuilder, namespaces::EthNamespaceClient};

pub async fn get_sync_status(pool: ConnectionPool, rpc_url: &str) -> Result<bool, Error> {
    let mut connection = pool.access_storage().await.unwrap();
    let client = HttpClientBuilder::default()
        .build(rpc_url)
        .expect("faile to build rpc client");

    let local_block_number = connection
        .blocks_web3_dal()
        .get_sealed_miniblock_number()
        .await
        .map(|n| U64::from(n.0))
        .map_err(|e| {
            tracing::error!("failed to get local block number ");
            e
        })?;

    let latest_block_number = client.get_block_number().await.map_err(|e| {
        tracing::error!("failed to get latest block number ");
        e
    })?;

    tracing::debug!(
        " local_block_number {:?}  latest_block_number {:?}",
        local_block_number,
        latest_block_number
    );

    Ok(local_block_number.eq(&latest_block_number))
}
