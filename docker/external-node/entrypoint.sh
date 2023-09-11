#!/bin/bash
set -a
# start server
source /etc/env/ext-node.env
source /etc/env/.init.env
source /etc/env/user.env

micro_external_node $*