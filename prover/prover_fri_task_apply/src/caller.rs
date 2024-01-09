use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use micro_types::{web3::types::Log, H256};

use crate::client::Error;

#[derive(Debug)]
pub struct Caller {
    sender: mpsc::UnboundedSender<Data>,
}

impl Caller {
    pub fn new(sender: mpsc::UnboundedSender<Data>) -> Caller {
        Caller { sender }
    }

    pub async fn query_and_send(&mut self, data: u32) -> Result<Result<H256, Error>> {
        let (callback, tr) = oneshot::channel::<Result<H256, Error>>();

        self.sender.unbounded_send(Data::List(data, callback))?;

        Ok(tr.await?)
    }

    pub async fn scan_event_apply(&mut self, data: Vec<Log>) -> Result<Result<H256, Error>> {
        let (callback, tr) = oneshot::channel::<Result<H256, Error>>();

        self.sender.unbounded_send(Data::Apply(data, callback))?;

        Ok(tr.await?)
    }
}

pub enum Data {
    List(u32, oneshot::Sender<Result<H256, Error>>),
    Apply(Vec<Log>, oneshot::Sender<Result<H256, Error>>),
}
