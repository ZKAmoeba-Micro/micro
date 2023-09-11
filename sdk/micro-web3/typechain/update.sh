#!/bin/bash

cd `dirname $0`

cp -f $MICRO_HOME/contracts/ethereum/typechain/{IMicro,IL2Bridge,IL1Bridge,IERC20Metadata,IAllowList}.d.ts .
cp -f $MICRO_HOME/contracts/ethereum/typechain/{IMicro,IL2Bridge,IL1Bridge,IERC20Metadata,IAllowList}Factory.ts .
