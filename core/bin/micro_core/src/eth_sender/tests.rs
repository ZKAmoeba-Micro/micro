use crate::eth_sender::{Aggregator, EthTxAggregator, EthTxManager};
use crate::l1_gas_price::GasAdjuster;
use db_test_macro::db_test;
use micro_config::{
    configs::eth_sender::{ProofSendingMode, SenderConfig},
    ETHSenderConfig, GasAdjusterConfig, MicroConfig,
};
use micro_dal::{ConnectionPool, StorageProcessor};
use micro_eth_client::{clients::mock::MockEthereum, EthInterface};
use micro_types::{
    aggregated_operations::{AggregatedOperation, BlocksExecuteOperation},
    Address, L1BlockNumber,
};

use crate::api_server::tx_sender::{TxSender, TxSenderBuilder};
use crate::api_server::web3;
use crate::l1_gas_price::{BoundedGasAdjuster, L1GasPriceProvider};
use std::sync::Arc;

// Alias to conveniently call static methods of ETHSender.
type MockEthTxManager = EthTxManager<
    Arc<MockEthereum>,
    GasAdjuster<Arc<MockEthereum>>,
    BoundedGasAdjuster<GasAdjuster<Arc<MockEthereum>>>,
>;

const DUMMY_OPERATION: AggregatedOperation =
    AggregatedOperation::ExecuteBlocks(BlocksExecuteOperation { blocks: vec![] });

#[derive(Debug)]
struct EthSenderTester {
    conn: ConnectionPool,
    gateway: Arc<MockEthereum>,
    manager: MockEthTxManager,
    aggregator: EthTxAggregator,
    gas_adjuster: Arc<GasAdjuster<Arc<MockEthereum>>>,
}

impl EthSenderTester {
    const WAIT_CONFIRMATIONS: u64 = 10;
    const MAX_BASE_FEE_SAMPLES: usize = 7;

    async fn new(connection_pool: ConnectionPool, history: Vec<u64>) -> Self {
        let eth_sender_config = ETHSenderConfig::from_env();
        let aggregator_config = SenderConfig {
            aggregated_proof_sizes: vec![1],
            ..eth_sender_config.sender.clone()
        };
        let config = MicroConfig::from_env();
        let replica_connection_pool = ConnectionPool::new(None, false);

        let gateway = Arc::new(MockEthereum::default().with_fee_history(history));
        let gas_adjuster = Arc::new(
            GasAdjuster::new(
                gateway.clone(),
                GasAdjusterConfig {
                    max_base_fee_samples: Self::MAX_BASE_FEE_SAMPLES,
                    pricing_formula_parameter_a: 3.0,
                    pricing_formula_parameter_b: 2.0,
                    ..eth_sender_config.gas_adjuster
                },
            )
            .await
            .unwrap(),
        );

        let bounded_gas_adjuster = Arc::new(BoundedGasAdjuster::new(
            config.chain.state_keeper.max_l1_gas_price(),
            gas_adjuster.clone(),
        ));

        let tx_sender = build_tx_sender(
            &config,
            replica_connection_pool.clone(),
            connection_pool.clone(),
            bounded_gas_adjuster,
        );
        let builder_state = web3::ApiBuilder::jsonrpc_backend(
            config.clone().into(),
            replica_connection_pool.clone(),
        )
        .http(config.api.web3_json_rpc.http_port)
        .with_filter_limit(config.api.web3_json_rpc.filters_limit())
        .with_threads(config.api.web3_json_rpc.threads_per_server as usize)
        .with_tx_sender(tx_sender)
        .build_rpc_state();

        let aggregator = EthTxAggregator::new(
            SenderConfig {
                wait_confirmations: Self::WAIT_CONFIRMATIONS,
                proof_sending_mode: ProofSendingMode::SkipEveryProof,
                ..eth_sender_config.sender.clone()
            },
            // Aggregator - unused
            Aggregator::new(aggregator_config.clone()),
            // micro contract address
            Address::random(),
            0,
        );

        let manager = EthTxManager::new(
            SenderConfig {
                wait_confirmations: Self::WAIT_CONFIRMATIONS,
                ..eth_sender_config.sender
            },
            gas_adjuster.clone(),
            gateway.clone(),
            builder_state,
        );
        Self {
            gateway,
            manager,
            aggregator,
            gas_adjuster,
            conn: connection_pool,
        }
    }

    async fn storage(&self) -> StorageProcessor<'static> {
        self.conn.access_test_storage().await
    }
}

fn build_tx_sender<G: L1GasPriceProvider>(
    config: &MicroConfig,
    replica_pool: ConnectionPool,
    master_pool: ConnectionPool,
    l1_gas_price_provider: Arc<G>,
) -> TxSender<G> {
    let mut tx_sender_builder = TxSenderBuilder::new(config.clone().into(), replica_pool)
        .with_main_connection_pool(master_pool)
        .with_state_keeper_config(config.chain.state_keeper.clone());

    // Add rate limiter if enabled.
    if let Some(transactions_per_sec_limit) = config.api.web3_json_rpc.transactions_per_sec_limit {
        tx_sender_builder = tx_sender_builder.with_rate_limiter(transactions_per_sec_limit);
    };

    tx_sender_builder.build(
        l1_gas_price_provider,
        config.chain.state_keeper.default_aa_hash,
    )
}

// Tests that we send multiple transactions and confirm them all in one iteration.
#[db_test]
async fn confirm_many(connection_pool: ConnectionPool) -> anyhow::Result<()> {
    let mut tester = EthSenderTester::new(connection_pool.clone(), vec![10; 100]).await;

    let mut hashes = vec![];

    for _ in 0..5 {
        let tx = tester
            .aggregator
            .save_eth_tx(&mut tester.storage().await, &DUMMY_OPERATION)?;
        let hash = tester
            .manager
            .send_eth_tx(
                &mut tester.storage().await,
                &tx,
                0,
                L1BlockNumber(tester.gateway.block_number("").await?.as_u32()),
            )
            .await?;
        hashes.push(hash);
    }

    // check that we sent something
    assert_eq!(tester.gateway.sent_txs.read().unwrap().len(), 5);
    assert_eq!(
        tester
            .storage()
            .await
            .eth_sender_dal()
            .get_inflight_txs()
            .len(),
        5
    );

    for hash in hashes {
        tester
            .gateway
            .execute_tx(hash, true, EthSenderTester::WAIT_CONFIRMATIONS)?;
    }

    let to_resend = tester
        .manager
        .monitor_inflight_transactions(
            &mut tester.storage().await,
            L1BlockNumber(tester.gateway.block_number("a").await.unwrap().as_u32()),
        )
        .await?;

    // check that transaction is marked as accepted
    assert_eq!(
        tester
            .storage()
            .await
            .eth_sender_dal()
            .get_inflight_txs()
            .len(),
        0
    );

    // also check that we didn't try to resend it
    assert!(to_resend.is_none());

    Ok(())
}

// Tests that we resend first unmined transaction every block with an increased gas price.
#[db_test]
async fn resend_each_block(connection_pool: ConnectionPool) -> anyhow::Result<()> {
    let mut tester = EthSenderTester::new(connection_pool.clone(), vec![7, 6, 5, 4, 3, 2, 1]).await;

    // after this, median should be 6
    tester.gateway.advance_block_number(3);
    tester.gas_adjuster.keep_updated().await?;

    let block = L1BlockNumber(tester.gateway.block_number("").await?.as_u32());
    let tx = tester
        .aggregator
        .save_eth_tx(&mut tester.storage().await, &DUMMY_OPERATION)?;

    let hash = tester
        .manager
        .send_eth_tx(&mut tester.storage().await, &tx, 0, block)
        .await?;

    // check that we sent something and stored it in the db
    assert_eq!(tester.gateway.sent_txs.read().unwrap().len(), 1);
    assert_eq!(
        tester
            .storage()
            .await
            .eth_sender_dal()
            .get_inflight_txs()
            .len(),
        1
    );

    let sent_tx = tester.gateway.sent_txs.read().unwrap()[&hash];
    assert_eq!(sent_tx.hash, hash);
    assert_eq!(sent_tx.nonce, 0);
    assert_eq!(sent_tx.base_fee.as_usize(), 18); // 6 * 3 * 2^0

    // now, median is 5
    tester.gateway.advance_block_number(2);
    tester.gas_adjuster.keep_updated().await?;
    let block = L1BlockNumber(tester.gateway.block_number("").await?.as_u32());

    let (to_resend, _) = tester
        .manager
        .monitor_inflight_transactions(&mut tester.storage().await, block)
        .await?
        .unwrap();

    let resent_hash = tester
        .manager
        .send_eth_tx(&mut tester.storage().await, &to_resend, 1, block)
        .await?;

    // check that transaction has been resent
    assert_eq!(tester.gateway.sent_txs.read().unwrap().len(), 2);
    assert_eq!(
        tester
            .storage()
            .await
            .eth_sender_dal()
            .get_inflight_txs()
            .len(),
        1
    );

    let resent_tx = tester.gateway.sent_txs.read().unwrap()[&resent_hash];
    assert_eq!(resent_tx.nonce, 0);
    assert_eq!(resent_tx.base_fee.as_usize(), 30); // 5 * 3 * 2^1

    Ok(())
}

// Tests that if transaction was mined, but not enough blocks has been mined since,
// we won't mark it as confirmed but also won't resend it.
#[db_test]
async fn dont_resend_already_mined(connection_pool: ConnectionPool) -> anyhow::Result<()> {
    let mut tester = EthSenderTester::new(connection_pool.clone(), vec![100; 100]).await;
    let tx = tester
        .aggregator
        .save_eth_tx(&mut tester.storage().await, &DUMMY_OPERATION)
        .unwrap();

    let hash = tester
        .manager
        .send_eth_tx(
            &mut tester.storage().await,
            &tx,
            0,
            L1BlockNumber(tester.gateway.block_number("").await.unwrap().as_u32()),
        )
        .await
        .unwrap();

    // check that we sent something and stored it in the db
    assert_eq!(tester.gateway.sent_txs.read().unwrap().len(), 1);
    assert_eq!(
        tester
            .storage()
            .await
            .eth_sender_dal()
            .get_inflight_txs()
            .len(),
        1
    );

    // mine the transaction but don't have enough confirmations yet
    tester
        .gateway
        .execute_tx(hash, true, EthSenderTester::WAIT_CONFIRMATIONS - 1)?;

    let to_resend = tester
        .manager
        .monitor_inflight_transactions(
            &mut tester.storage().await,
            L1BlockNumber(tester.gateway.block_number("a").await.unwrap().as_u32()),
        )
        .await?;

    // check that transaction is still considered inflight
    assert_eq!(
        tester
            .storage()
            .await
            .eth_sender_dal()
            .get_inflight_txs()
            .len(),
        1
    );

    // also check that we didn't try to resend it
    assert!(to_resend.is_none());

    Ok(())
}

#[db_test]
async fn three_scenarios(connection_pool: ConnectionPool) -> anyhow::Result<()> {
    let mut tester = EthSenderTester::new(connection_pool.clone(), vec![100; 100]).await;
    let mut hashes = vec![];

    for _ in 0..3 {
        let tx = tester
            .aggregator
            .save_eth_tx(&mut tester.storage().await, &DUMMY_OPERATION)
            .unwrap();

        let hash = tester
            .manager
            .send_eth_tx(
                &mut tester.storage().await,
                &tx,
                0,
                L1BlockNumber(tester.gateway.block_number("").await.unwrap().as_u32()),
            )
            .await
            .unwrap();

        hashes.push(hash);
    }

    // check that we sent something
    assert_eq!(tester.gateway.sent_txs.read().unwrap().len(), 3);

    // mined & confirmed
    tester
        .gateway
        .execute_tx(hashes[0], true, EthSenderTester::WAIT_CONFIRMATIONS)?;

    // mined but not confirmed
    tester
        .gateway
        .execute_tx(hashes[1], true, EthSenderTester::WAIT_CONFIRMATIONS - 1)?;

    let (to_resend, _) = tester
        .manager
        .monitor_inflight_transactions(
            &mut tester.storage().await,
            L1BlockNumber(tester.gateway.block_number("a").await.unwrap().as_u32()),
        )
        .await?
        .expect("we should be trying to resend the last tx");

    // check that last 2 transactions are still considered inflight
    assert_eq!(
        tester
            .storage()
            .await
            .eth_sender_dal()
            .get_inflight_txs()
            .len(),
        2
    );

    // last sent transaction has nonce == 2, because they start from 0
    assert_eq!(to_resend.nonce, 2);

    Ok(())
}

#[should_panic(expected = "We can't operate after tx fail")]
#[db_test]
async fn failed_eth_tx(connection_pool: ConnectionPool) {
    let mut tester = EthSenderTester::new(connection_pool.clone(), vec![100; 100]).await;

    let tx = tester
        .aggregator
        .save_eth_tx(&mut tester.storage().await, &DUMMY_OPERATION)
        .unwrap();

    let hash = tester
        .manager
        .send_eth_tx(
            &mut tester.storage().await,
            &tx,
            0,
            L1BlockNumber(tester.gateway.block_number("").await.unwrap().as_u32()),
        )
        .await
        .unwrap();

    // fail this tx
    tester
        .gateway
        .execute_tx(hash, false, EthSenderTester::WAIT_CONFIRMATIONS)
        .unwrap();
    tester
        .manager
        .monitor_inflight_transactions(
            &mut tester.storage().await,
            L1BlockNumber(tester.gateway.block_number("a").await.unwrap().as_u32()),
        )
        .await
        .unwrap();
}
