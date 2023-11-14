use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use micro_config::configs::eth_sender::SenderConfig;
use micro_eth_signer::TransactionParameters;
use micro_types::{ethabi::Bytes, transaction_request::CallRequest};
use tokio::sync::watch;

use self::caller::Data;

pub mod caller;

pub struct L2Sender {
    config: SenderConfig,

    msg_receiver: mpsc::UnboundedReceiver<caller::Data>,
    msg_sender: mpsc::UnboundedSender<caller::Data>,
}

impl L2Sender {
    pub fn new(config: SenderConfig) -> Self {
        let (tx, rx) = mpsc::unbounded();
        L2Sender {
            config,
            msg_receiver: rx,
            msg_sender: tx,
        }
    }

    pub fn get_caller(&self) -> caller::Caller {
        caller::Caller::new(self.msg_sender.clone())
    }

    pub async fn run(mut self, mut stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()> {
        loop {
            if *stop_receiver.borrow_and_update() {
                tracing::info!("Stop signal received, metadata_calculator is shutting down");
                break;
            }

            tokio::select! {
                _ = stop_receiver.changed() => {
                    tracing::info!("Stop signal received, metadata_calculator is shutting down");
                    break;
                }
                data = self.msg_receiver.next() => {
                    if let Some(data) = data {
                        self.process(data).await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn process(&self, data: Data) -> anyhow::Result<()> {
        match data {
            caller::Data::Call(data, callback) => self.call(data, callback).await,
            caller::Data::Send(data, callback) => self.send(data, callback).await,
        }
    }

    async fn call(
        &self,
        data: CallRequest,
        callback: oneshot::Sender<Bytes>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn send(
        &self,
        data: TransactionParameters,
        callback: oneshot::Sender<Bytes>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
