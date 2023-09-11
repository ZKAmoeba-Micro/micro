#!/bin/bash

cd `dirname $0`

# Main micro contract interface
cat $MICRO_HOME/contracts/ethereum/artifacts/cache/solpp-generated-contracts/micro/interfaces/IMicro.sol/IMicro.json | jq '{ abi: .abi}' > Micro.json
# Default L1 bridge
cat $MICRO_HOME/contracts/ethereum/artifacts/cache/solpp-generated-contracts/bridge/interfaces/IL1Bridge.sol/IL1Bridge.json | jq '{ abi: .abi}' > L1Bridge.json
# Paymaster interface
cat $MICRO_HOME/contracts/micro/artifacts-zk/cache-zk/solpp-generated-contracts/interfaces/IPaymasterFlow.sol/IPaymasterFlow.json | jq '{ abi: .abi}' > IPaymasterFlow.json
