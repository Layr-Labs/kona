#!/bin/bash

end=$1
start=$2

for ((i = $end ; i >= $start ; i--)); do
    hashResult=$(cast block $i --json --rpc-url http://127.0.0.1:8545 | jq -r .hash)
    echo "hash:  $hashResult     number:  $i"
done

