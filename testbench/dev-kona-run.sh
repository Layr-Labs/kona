set -euo pipefail
# set -x # echo commands to debug

# DEFINE PARAMETERS
#Path to the op-geth genesis file
L2_GENESIS=
# Path to rollup chain parameters (rollup.json)
ROLLUP_CONFIG=
# Endpoint for L1 execution client JSON-RPC server
L1_RPC=http://127.0.0.1:8545
# Endpoint for L2 execution client JSON-RPC server
L2_RPC=http://127.0.0.1:9545
# Endpoint for L1 beacon chain API server
L1_BEACON=http://127.0.0.1:5052

# Set claim at finalized block
L2_CLAIM_NUMBER=$(cast block finalized --json --rpc-url $L2_RPC | jq -r .number | cast 2d)
echo "L2_CLAIM_NUMBER: $L2_CLAIM_NUMBER"


# L2_CLAIM_NUMBER=145708 # $((L2_CLAIM_NUMBER1 - 100))

L2_CLAIM_STATE_ROOT=$(cast block $L2_CLAIM_NUMBER --json --rpc-url $L2_RPC | jq -r .stateRoot)
L2_CLAIM_MESSAGE_PASSER_STORAGE_ROOT=$(cast proof -B $L2_CLAIM_NUMBER --rpc-url $L2_RPC 0x4200000000000000000000000000000000000016 | jq -r .storageHash)
L2_CLAIM_BLOCK_HASH=$(cast block $L2_CLAIM_NUMBER --json --rpc-url $L2_RPC | jq -r .hash)
L2_CLAIM=$(cast keccak "0x0000000000000000000000000000000000000000000000000000000000000000${L2_CLAIM_STATE_ROOT#0x}${L2_CLAIM_MESSAGE_PASSER_STORAGE_ROOT#0x}${L2_CLAIM_BLOCK_HASH#0x}")

# Start one block prior to claim
L2_STARTING_BLOCK_NUM=$((L2_CLAIM_NUMBER - 1))

L2_START_STATE_ROOT=$(cast block $L2_STARTING_BLOCK_NUM --json --rpc-url $L2_RPC | jq -r .stateRoot)
L2_START_MESSAGE_PASSER_STORAGE_ROOT=$(cast proof -B $L2_STARTING_BLOCK_NUM --rpc-url $L2_RPC 0x4200000000000000000000000000000000000016 | jq -r .storageHash)
L2_START_BLOCK_HASH=$(cast block $L2_STARTING_BLOCK_NUM --json --rpc-url $L2_RPC | jq -r .hash)
L2_START_OUTPUT_ROOT=$(cast keccak "0x0000000000000000000000000000000000000000000000000000000000000000${L2_START_STATE_ROOT#0x}${L2_START_MESSAGE_PASSER_STORAGE_ROOT#0x}${L2_START_BLOCK_HASH#0x}")

echo "State root: $L2_START_STATE_ROOT"
echo "Message passer storage root: $L2_START_MESSAGE_PASSER_STORAGE_ROOT"
echo "Block hash: $L2_START_BLOCK_HASH"
echo "Claim: $L2_START_OUTPUT_ROOT"


# # L1 origin corresponding to starting block
# L1_ORIGIN_NUM=$(cast rpc "optimism_outputAtBlock" $(cast 2h $L2_STARTING_BLOCK_NUM) --rpc-url $OP_NODE | jq -r .blockRef.l1origin.number)
# # L1 head is advanced to account for the span of blocks that may contain txs for the claim block
# L1_HEAD_NUM=$((L1_ORIGIN_NUM + 30))
# L1_HEAD_HASH=$(cast block $L1_HEAD_NUM --json --rpc-url $L1_RPC | jq -r .hash)
# L1_HEAD_HASH=$(cast call -b $L2_STARTING_BLOCK_NUM "0x4200000000000000000000000000000000000015" "hash()")

L1_HEAD_NUM=$(cast bn --rpc-url $L1_RPC)

#L1_HEAD_NUM=338
L1_HEAD_HASH=$(cast block $L1_HEAD_NUM --json --rpc-url $L1_RPC | jq -r .hash)

# Print all gathered inputs
echo "===== [ PROGRAM INPUTS ] ====="
echo "L2 claim block number: $L2_CLAIM_NUMBER"
echo "L2 claim output root: $L2_CLAIM"
echo "Starting L2 output root: $L2_START_OUTPUT_ROOT"
echo "Starting L2 head: $L2_START_BLOCK_HASH (block #$L2_STARTING_BLOCK_NUM)"
# echo "Starting L2 output root's L1 origin: $L1_ORIGIN_NUM"
echo "L1 head (block #$L1_HEAD_NUM): $L1_HEAD_HASH"

OP_MAINNET=10
OP_SEPOLIA=11155420
OP_DEV=901

/home/ubuntu/kona/target/debug/kona-host \
    --native \
    --l1-node-address $L1_RPC \
    --l2-node-address $L2_RPC \
    --l1-beacon-address $L1_BEACON \
    --l1-head $L1_HEAD_HASH \
    --agreed-l2-head-hash $L2_START_BLOCK_HASH \
    --agreed-l2-output-root $L2_START_OUTPUT_ROOT \
    --claimed-l2-output-root $L2_CLAIM \
    --claimed-l2-block-number $L2_CLAIM_NUMBER \
    --data-dir ./tmp/op-db-altda \
    --rollup-config-path /home/ubuntu/op-main-repo/.devnet/rollup.json \
    -vvv

# L2 claim block number: 128534042
# L2 claim output root: 0x662e4b8af90bdc5e5f08ec0d9d40c63e621a9a73943e8dcbd23fb8385777776c
# Starting L2 output root: 0x481212a24150bc4901bb29f6e356982825bc99969d16d32ab9a3f3f82cfea726
# Starting L2 head: 0xa77a94d89f8a588883cb005b1e53d4035443bdcf2d83677558f8ec601450fdb3 (block #128534041)
# L1 head (block #21275598): 0x48458f748b391c09ac399d4aa358d3b89de53ec83691d31201f5fff7d46a7fd6
