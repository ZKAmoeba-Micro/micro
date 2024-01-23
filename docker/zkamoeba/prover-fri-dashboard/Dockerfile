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

RUN cd prover && cargo build --release --bin micro_prover_fri_dashboard

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y curl libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY prover/prover_fri_dashboard/ui/dist/ /ui/
COPY etc/system-contracts/artifacts-zk/cache-zk/solpp-generated-contracts/interfaces/IDeposit.sol/IDeposit.json /etc/system-contracts/artifacts-zk/cache-zk/solpp-generated-contracts/interfaces/IDeposit.sol/IDeposit.json
COPY etc/system-contracts/artifacts-zk/cache-zk/solpp-generated-contracts/interfaces/IWhiteList.sol/IWhiteList.json /etc/system-contracts/artifacts-zk/cache-zk/solpp-generated-contracts/interfaces/IWhiteList.sol/IWhiteList.json
COPY contracts/ethereum/artifacts/@openzeppelin/contracts/token/ERC20/ERC20.sol/ERC20.json /contracts/ethereum/artifacts/@openzeppelin/contracts/token/ERC20/ERC20.sol/ERC20.json

COPY --from=builder /usr/src/micro/prover/target/release/micro_prover_fri_dashboard /usr/bin/

ENTRYPOINT ["micro_prover_fri_dashboard"]
