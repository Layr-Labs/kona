#!/bin/bash

pushd /home/ubuntu/kona
just build-native
popd

rm -r ./tmp/op-db-altda

./dev-kona-run.sh | sed "s/\x1b\[[0-9;]*[mG]//g" 
