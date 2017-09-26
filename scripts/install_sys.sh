#!/bin/sh
set -e

# 1) install add-apt-repository
apt-get update -q
apt-get install -y software-properties-common

# 2) add libsodium repository if using trusty version
if [ $(lsb_release -s -c) = "trusty" ]; then
    add-apt-repository -y ppa:chris-lea/libsodium; 
fi;

# 3) add ethereum repository
add-apt-repository -y ppa:ethereum/ethereum

# 4) install dependencies
apt-get update -q
apt-get install -y build-essential \
        libsnappy-dev  capnproto  libgoogle-perftools-dev  libssl-dev libsodium* libzmq3-dev \
		pkg-config rabbitmq-server solc curl jq  google-perftools python-pip
