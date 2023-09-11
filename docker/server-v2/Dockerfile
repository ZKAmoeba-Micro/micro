# For using private GitHub dependencies, CI downdloads all crates outside of the contatiner
# Not expected to work locally

# syntax=docker/dockerfile:experimental
FROM rust:1.67-buster as builder
RUN apt-get update && apt-get install -y clang && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/micro
COPY . .

RUN CARGO_HOME=./cargo cargo build --release

FROM debian:buster-slim
RUN apt-get update && apt-get install -y curl libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
EXPOSE 3000
EXPOSE 3031
EXPOSE 3030
COPY --from=builder /usr/src/micro/target/release/micro_server /usr/bin
COPY --from=builder /usr/src/micro/target/release/block_reverter /usr/bin
COPY --from=builder /usr/src/micro/target/release/merkle_tree_consistency_checker /usr/bin
COPY --from=builder /usr/src/micro/target/release/rocksdb_util /usr/bin
COPY etc/system-contracts/bootloader/build/artifacts/ /etc/system-contracts/bootloader/build/artifacts/
COPY etc/system-contracts/contracts/artifacts/ /etc/system-contracts/contracts/artifacts/
COPY etc/system-contracts/contracts/precompiles/artifacts/ /etc/system-contracts/contracts/precompiles/artifacts/
COPY etc/system-contracts/artifacts-zk /etc/system-contracts/artifacts-zk
COPY contracts/ethereum/artifacts/ /contracts/ethereum/artifacts/
COPY contracts/micro/artifacts-zk/ /contracts/micro/artifacts-zk/
COPY etc/tokens/ /etc/tokens/
COPY etc/ERC20/ /etc/ERC20/
COPY core/bin/verification_key_generator_and_server/data/ /core/bin/verification_key_generator_and_server/data/

ENTRYPOINT ["micro_server"]
