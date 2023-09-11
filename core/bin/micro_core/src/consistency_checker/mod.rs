use micro_dal::ConnectionPool;
use micro_types::web3::{
    error, ethabi,
    transports::Http,
    types::{Address, TransactionId},
    Web3,
};
use micro_types::L1BatchNumber;
use std::time::Duration;

#[derive(Debug)]
pub struct ConsistencyChecker {
    // ABI of the micro contract
    contract: ethabi::Contract,
    // Address of the micro contract
    contract_addr: Address,
    // How many past batches to check when starting
    max_batches_to_recheck: u32,
    web3: Web3<Http>,
    db: ConnectionPool,
}

const SLEEP_DELAY: Duration = Duration::from_secs(5);

impl ConsistencyChecker {
    pub fn new(
        web3_url: &str,
        contract_addr: Address,
        max_batches_to_recheck: u32,
        db: ConnectionPool,
    ) -> Self {
        let web3 = Web3::new(Http::new(web3_url).unwrap());
        let contract = micro_contracts::micro_contract();
        Self {
            web3,
            contract,
            contract_addr,
            max_batches_to_recheck,
            db,
        }
    }

    async fn check_commitments(&self, batch_number: L1BatchNumber) -> Result<bool, error::Error> {
        let mut storage = self.db.access_storage_blocking();

        let storage_block = storage
            .blocks_dal()
            .get_storage_block(batch_number)
            .unwrap_or_else(|| panic!("Block {} not found in the database", batch_number));

        let commit_tx_id = storage_block
            .eth_commit_tx_id
            .unwrap_or_else(|| panic!("Block commit tx not found for block {}", batch_number))
            as u32;

        let block_metadata = storage
            .blocks_dal()
            .get_block_with_metadata(storage_block)
            .unwrap_or_else(|| {
                panic!(
                    "Block metadata for block {} not found in the database",
                    batch_number
                )
            });

        let commit_tx_hash = storage
            .eth_sender_dal()
            .get_confirmed_tx_hash_by_eth_tx_id(commit_tx_id)
            .unwrap_or_else(|| {
                panic!(
                    "Commit tx hash not found in the database. Commit tx id: {}",
                    commit_tx_id
                )
            });

        vlog::info!(
            "Checking commit tx {} for batch {}",
            commit_tx_hash,
            batch_number.0
        );

        // we can't get tx calldata from db because it can be fake
        let commit_tx = self
            .web3
            .eth()
            .transaction(TransactionId::Hash(commit_tx_hash))
            .await?
            .expect("Commit tx not found on L1");

        let commit_tx_status = self
            .web3
            .eth()
            .transaction_receipt(commit_tx_hash)
            .await?
            .expect("Commit tx receipt not found on L1")
            .status;

        assert_eq!(
            commit_tx_status,
            Some(1.into()),
            "Main node gave us a failed commit tx"
        );
        assert_eq!(
            commit_tx.to,
            Some(self.contract_addr),
            "Main node gave us a commit tx sent to a wrong address"
        );

        let commitments = self
            .contract
            .function("commitBlocks")
            .unwrap()
            .decode_input(&commit_tx.input.0[4..])
            .unwrap()
            .pop()
            .unwrap()
            .into_array()
            .unwrap();

        // Commit transactions usually publish multiple commitments at once, so we need to find
        // the one that corresponds to the batch we're checking.
        let first_batch_number = match &commitments[0] {
            ethabi::Token::Tuple(tuple) => tuple[0].clone().into_uint().unwrap().as_usize(),
            _ => panic!("ABI does not match the commitBlocks() function on the micro contract"),
        };
        let commitment = &commitments[batch_number.0 as usize - first_batch_number];

        Ok(commitment == &block_metadata.l1_commit_data())
    }

    fn last_committed_batch(&self) -> L1BatchNumber {
        self.db
            .access_storage_blocking()
            .blocks_dal()
            .get_number_of_last_block_committed_on_eth()
            .unwrap_or(L1BatchNumber(0))
    }

    pub async fn run(self, stop_receiver: tokio::sync::watch::Receiver<bool>) {
        let mut batch_number: L1BatchNumber = self
            .last_committed_batch()
            .0
            .saturating_sub(self.max_batches_to_recheck)
            .max(1)
            .into();

        vlog::info!("Starting consistency checker from batch {}", batch_number.0);

        loop {
            if *stop_receiver.borrow() {
                vlog::info!("Stop signal received, consistency_checker is shutting down");
                break;
            }

            let batch_has_metadata = self
                .db
                .access_storage_blocking()
                .blocks_dal()
                .get_block_metadata(batch_number)
                .is_some();

            // The batch might be already committed but not yet processed by the external node's tree
            // OR the batch might be processed by the external node's tree but not yet committed.
            // We need both.
            if !batch_has_metadata || self.last_committed_batch() < batch_number {
                tokio::time::sleep(SLEEP_DELAY).await;
                continue;
            }

            match self.check_commitments(batch_number).await {
                Ok(true) => {
                    vlog::info!("Batch {} is consistent with L1", batch_number.0);
                    metrics::gauge!(
                        "external_node.last_correct_batch",
                        batch_number.0 as f64,
                        "component" => "consistency_checker",
                    );
                    batch_number.0 += 1;
                }
                Ok(false) => {
                    panic!("Batch {} is inconsistent with L1", batch_number.0);
                }
                Err(e) => {
                    vlog::warn!("Consistency checker error: {}", e);
                    tokio::time::sleep(SLEEP_DELAY).await;
                }
            }
        }
    }
}
