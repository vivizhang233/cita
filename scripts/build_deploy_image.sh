#!/bin/sh
# image usage:
# 1) generate configuration
# docker run -ti -v $PWD:/data cryptape/cita /cita/admintool/admintool.sh
# 2) run whole node in one instance
# docker run -ti -v $PWD:/data cryptape/cita bash -c "/cita/admintool/cita start node0"
# 3) or run single service in one instance
# docker run -ti -v $PWD:/data cryptape/cita bash -c "/cita/bin/chain -c config.json -g genesis.json"
cd ..
docker run -ti -v ${PWD}:/source cryptape/cita:build bash -c  "scripts/config_rabbitmq.sh; make release"
docker build --tag cryptape/cita --file scripts/Dockerfile-run .
cd scripts
