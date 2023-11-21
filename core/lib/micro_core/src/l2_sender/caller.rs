use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use micro_types::{transaction_request::CallRequest, H256};
use micro_web3_decl::error::Web3Error;

#[derive(Debug)]
pub struct Caller {
    sender: mpsc::UnboundedSender<Data>,
}

impl Caller {
    pub fn new(sender: mpsc::UnboundedSender<Data>) -> Caller {
        Caller { sender }
    }

    pub async fn call(&mut self, data: CallRequest) -> Result<Result<Vec<u8>, Web3Error>> {
        let (callback, tr) = oneshot::channel::<Result<Vec<u8>, Web3Error>>();

        self.sender.unbounded_send(Data::Call(data, callback))?;

        Ok(tr.await?)
    }

    pub async fn send(&mut self, data: CallRequest) -> Result<Result<H256, Web3Error>> {
        let (callback, tr) = oneshot::channel::<Result<H256, Web3Error>>();

        self.sender.unbounded_send(Data::Send(data, callback))?;

        Ok(tr.await?)
    }
}

pub enum Data {
    Call(CallRequest, oneshot::Sender<Result<Vec<u8>, Web3Error>>),
    Send(CallRequest, oneshot::Sender<Result<H256, Web3Error>>),
}
