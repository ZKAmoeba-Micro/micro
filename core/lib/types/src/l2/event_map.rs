use crate::api::Log;
pub trait EventMapBuilder {
    type A;
    fn build_map(logs: Vec<Log>) -> Vec<Self::A>;
}
