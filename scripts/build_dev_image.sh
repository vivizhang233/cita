#!/bin/sh
# image usage:
# docker run -ti -v ${PWD}:/source cryptape/cita:build bash -c  "scripts/config_rabbitmq.sh; make release"
cd ..
docker build --tag cryptape/cita:build --file scripts/Dockerfile-build .
cd scripts
