use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use micro_eth_signer::TransactionParameters;
use micro_types::{ethabi::Bytes, transaction_request::CallRequest};

pub struct Caller {
    sender: mpsc::UnboundedSender<Data>,
}

impl Caller {
    pub fn new(sender: mpsc::UnboundedSender<Data>) -> Caller {
        Caller { sender }
    }

    pub async fn call(&mut self, data: CallRequest) -> Result<Bytes> {
        let (callback, tr) = oneshot::channel::<Bytes>();

        self.sender.unbounded_send(Data::Call(data, callback))?;

        Ok(tr.await?)
    }

    pub async fn send(&mut self, data: TransactionParameters) -> Result<Bytes> {
        let (callback, tr) = oneshot::channel::<Bytes>();

        self.sender.unbounded_send(Data::Send(data, callback))?;

        Ok(tr.await?)
    }
}

pub enum Data {
    Call(CallRequest, oneshot::Sender<Bytes>),
    Send(TransactionParameters, oneshot::Sender<Bytes>),
}
