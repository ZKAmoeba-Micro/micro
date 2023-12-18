//! Generates rust code from protobufs.
fn main() {
    micro_protobuf_build::Config {
        input_root: "src/consensus/proto".into(),
        proto_root: "micro/core/consensus".into(),
        dependencies: vec![],
        protobuf_crate: "::micro_protobuf".parse().unwrap(),
        is_public: false,
    }
    .generate()
    .expect("generate()");
}
