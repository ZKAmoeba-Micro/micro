FROM debian:bookworm-slim as builder

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y curl clang openssl libssl-dev gcc g++ \
    pkg-config build-essential libclang-dev ssh git && \
    rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y && \
    rustup install nightly-2023-07-21 && \
    rustup default nightly-2023-07-21

ARG SSH_KEY

RUN mkdir -p /root/.ssh/ && \
    echo "$SSH_KEY" > /root/.ssh/id_rsa && \
    chmod -R 600 /root/.ssh/ && \
    ssh-keyscan -t rsa github.com >> ~/.ssh/known_hosts

RUN git config --global url."ssh://git@github.com/".insteadOf https://github.com/

WORKDIR /usr/src/micro
COPY . .

RUN cd prover && cargo build --release --bin micro_prover_fri_task_apply

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y curl libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/micro/prover/target/release/micro_prover_fri_task_apply /usr/bin/

COPY contracts/ethereum/artifacts/cache/solpp-generated-contracts/governance/IGovernance.sol/IGovernance.json /contracts/ethereum/artifacts/cache/solpp-generated-contracts/governance/IGovernance.sol/IGovernance.json
COPY contracts/ethereum/artifacts/cache/solpp-generated-contracts/micro/interfaces/IMicro.sol/IMicro.json /contracts/ethereum/artifacts/cache/solpp-generated-contracts/micro/interfaces/IMicro.sol/IMicro.json
COPY contracts/ethereum/artifacts/cache/solpp-generated-contracts/dev-contracts/Multicall3.sol/Multicall3.json /contracts/ethereum/artifacts/cache/solpp-generated-contracts/dev-contracts/Multicall3.sol/Multicall3.json
COPY contracts/ethereum/artifacts/cache/solpp-generated-contracts/micro/Verifier.sol/Verifier.json /contracts/ethereum/artifacts/cache/solpp-generated-contracts/micro/Verifier.sol/Verifier.json
COPY contracts/micro/artifacts-zk/cache-zk/solpp-generated-contracts/bridge/interfaces/IL2Bridge.sol/IL2Bridge.json /contracts/micro/artifacts-zk/cache-zk/solpp-generated-contracts/bridge/interfaces/IL2Bridge.sol/IL2Bridge.json
COPY etc/ERC20/artifacts-zk/contracts/interfaces/IERC20.sol/IERC20.json /contracts/ethereum/artifacts/cache/solpp-generated-contracts/common/interfaces/IERC20.sol/IERC20.json
COPY etc/contracts-test-data/artifacts-zk/contracts/loadnext/loadnext_contract.sol/LoadnextContract.json /etc/contracts-test-data/artifacts-zk/contracts/loadnext/loadnext_contract.sol/LoadnextContract.json
COPY etc/contracts-test-data/artifacts-zk/contracts/loadnext/loadnext_contract.sol/Foo.json /etc/contracts-test-data/artifacts-zk/contracts/loadnext/loadnext_contract.sol/Foo.json
COPY etc/system-contracts/artifacts-zk/cache-zk/solpp-generated-contracts/interfaces/IDeposit.sol/IDeposit.json /etc/system-contracts/artifacts-zk/cache-zk/solpp-generated-contracts/interfaces/IDeposit.sol/IDeposit.json
COPY etc/system-contracts/artifacts-zk/cache-zk/solpp-generated-contracts/interfaces/IAssignment.sol/IAssignment.json /etc/system-contracts/artifacts-zk/cache-zk/solpp-generated-contracts/interfaces/IAssignment.sol/IAssignment.json

ENTRYPOINT ["micro_prover_fri_task_apply"]
