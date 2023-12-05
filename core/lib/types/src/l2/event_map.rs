use crate::{api::Log, Address};
use std::collections::HashMap;
pub trait EventMapBuilder {
    type A;
    fn build_map(logs: Vec<Log>) -> HashMap<Address, Self::A>;
}
