use futures::channel::mpsc;
use micro_types::web3::types::Log;

#[derive(Debug)]
pub struct Caller {
    sender: mpsc::UnboundedSender<Data>,
}

impl Caller {
    pub fn new(sender: mpsc::UnboundedSender<Data>) -> Caller {
        Caller { sender }
    }

    pub async fn query_and_apply(&mut self, data: u32) {
        let _ = self.sender.unbounded_send(Data::QueryApply(data));
    }

    pub async fn scan_event_apply(&mut self, data: Vec<Log>) {
        let _ = self.sender.unbounded_send(Data::EventApply(data));
    }
}

pub enum Data {
    QueryApply(u32),
    EventApply(Vec<Log>),
}
