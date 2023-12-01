#!/bin/bash

set -e
FRI_PROVER_SETUP_DATA_PATH=/usr/src/setup-data
if [ ! -d "$FRI_PROVER_SETUP_DATA_PATH" ]; then  
    mkdir -p "$FRI_PROVER_SETUP_DATA_PATH"  
fi 
echo "seq 1-13 start"
cd /prover  && for i in $(seq 1 13);  do  ./micro_setup_data_generator_fri --numeric-circuit $i --is_base_layer; done;
echo "seq 1-13 end"
echo "seq 1-15 start"
cd /prover  && for i in $(seq 1 15);  do  ./micro_setup_data_generator_fri --numeric-circuit $i; done;
echo "seq 1-15 end"