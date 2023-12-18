//! Generates rust code from protobufs.
fn main() {
    micro_protobuf_build::Config {
        input_root: "src/models/proto".into(),
        proto_root: "micro/dal".into(),
        dependencies: vec!["::micro_consensus_roles::proto".parse().unwrap()],
        protobuf_crate: "::micro_protobuf".parse().unwrap(),
        is_public: true,
    }
    .generate()
    .expect("generate()");
}
