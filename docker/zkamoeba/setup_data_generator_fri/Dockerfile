FROM nvidia/cuda:12.2.0-devel-ubuntu22.04 as builder

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y curl clang openssl libssl-dev gcc g++ \
    pkg-config build-essential libclang-dev ssh git && \
    rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# Building for Nvidia L4
ARG CUDA_ARCH=75
ENV CUDAARCHS=${CUDA_ARCH}

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y && \
    rustup install nightly-2023-07-21 && \
    rustup default nightly-2023-07-21

RUN curl -Lo cmake-3.24.2-linux-x86_64.sh https://github.com/Kitware/CMake/releases/download/v3.24.2/cmake-3.24.2-linux-x86_64.sh && \
    chmod +x cmake-3.24.2-linux-x86_64.sh && \
    ./cmake-3.24.2-linux-x86_64.sh --skip-license --prefix=/usr/local

ARG SSH_KEY

RUN mkdir -p /root/.ssh/ && \
    echo "$SSH_KEY" > /root/.ssh/id_rsa && \
    chmod -R 600 /root/.ssh/ && \
    ssh-keyscan -t rsa github.com >> ~/.ssh/known_hosts

RUN git config --global url."ssh://git@github.com/".insteadOf https://github.com/

WORKDIR /usr/src/micro
COPY . .

RUN cd prover && cargo build --release --features "gpu" --bin micro_setup_data_generator_fri

FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04

RUN apt-get update && apt-get install -y curl libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
# copy VK required for proof wrapping
COPY prover/vk_setup_data_generator_server_fri/data/ /prover/vk_setup_data_generator_server_fri/data/
COPY --from=builder /usr/src/micro/prover/target/release/micro_setup_data_generator_fri /prover
COPY --from=builder /usr/src/micro/docker/zkamoeba/setup_data_generator_fri/entrypoint.sh /prover

RUN chmod +x /prover/entrypoint.sh
ENTRYPOINT [ "sh", "/prover/entrypoint.sh"]

