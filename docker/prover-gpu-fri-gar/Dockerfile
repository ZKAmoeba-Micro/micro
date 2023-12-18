ARG PROVER_IMAGE
FROM us-docker.pkg.dev/zkamoeba-infra/zkamoeba-docker/prover-gpu-fri:2.0-$PROVER_IMAGE as prover

FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04 as app

# HACK copying to root is the only way to make Docker layer caching work for these files for some reason
COPY *.bin /

RUN apt-get update && apt-get install -y libpq5 ca-certificates openssl && rm -rf /var/lib/apt/lists/*

# copy finalization hints required for assembly generation
COPY --from=prover prover/vk_setup_data_generator_server_fri/data/ /prover/vk_setup_data_generator_server_fri/data/
COPY --from=prover /usr/bin/micro_prover_fri /usr/bin/

ENTRYPOINT ["micro_prover_fri"]
