#!/usr/bin/env bash
set -euo pipefail

echo "Building Anchor programs..."
anchor build

echo "Deploying programs to devnet..."
anchor deploy --provider.cluster devnet

echo "Run 'anchor keys list' and copy program IDs to lime-mvp .env:"
echo "VITE_LIME_MARKET_PROGRAM_ID"
echo "VITE_LIME_VAULT_PROGRAM_ID"
echo "VITE_LIME_SETTLEMENT_PROGRAM_ID"
