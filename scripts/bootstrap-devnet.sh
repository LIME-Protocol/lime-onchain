#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

DEPLOY_KP="${DEPLOY_KP:-${ANCHOR_WALLET:-${SOLANA_KEYPAIR:-$HOME/.config/solana/id.json}}}"
export ANCHOR_WALLET="$DEPLOY_KP"
export SOLANA_KEYPAIR="${SOLANA_KEYPAIR:-$DEPLOY_KP}"

echo "Using deploy wallet: $DEPLOY_KP"

echo "[1/3] Building programs"
anchor build

echo "[2/3] Deploying to devnet"
anchor deploy --provider.cluster devnet --provider.wallet "$DEPLOY_KP"

echo "[3/3] Initializing protocols and smoke market"
node scripts/bootstrap-devnet.mjs
