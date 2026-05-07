#!/bin/bash

# Process Pending Redemptions Script
# This script helps admins process pending redemptions

set -e

# Load environment variables
source .env

# Configuration
ASSET_ADDRESS="0xd73a79f6dabafce455ee3b921b6d4580bf73ecea"
INVESTOR_WALLET="0x6a1b1bfb6d79135ac3a16c01fe0535c3d334f332"

echo "🔍 Checking pending redemptions..."
echo "=================================="

# Check pending redemption amount
PENDING=$(cast call $ASSET_ADDRESS \
  "pendingRedemptionOf(address)(uint256)" \
  $INVESTOR_WALLET \
  --rpc-url $MONAD_RPC_URL)

echo "Asset: $ASSET_ADDRESS"
echo "Investor: $INVESTOR_WALLET"
echo "Pending Redemption: $PENDING"

if [ "$PENDING" == "0" ]; then
  echo "✅ No pending redemptions to process"
  exit 0
fi

# Calculate USDC value
REDEMPTION_PRICE=$(cast call $ASSET_ADDRESS \
  "redemptionPricePerToken()(uint256)" \
  --rpc-url $MONAD_RPC_URL)

echo "Redemption Price: $REDEMPTION_PRICE per token"

# Check treasury liquidity
TREASURY_ADDRESS=$(cast call $ASSET_ADDRESS \
  "treasury()(address)" \
  --rpc-url $MONAD_RPC_URL)

echo "Treasury: $TREASURY_ADDRESS"

# Prompt for confirmation
echo ""
echo "⚠️  This will process the redemption and transfer USDC to the investor"
read -p "Do you want to proceed? (yes/no): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
  echo "❌ Cancelled"
  exit 0
fi

echo ""
echo "📤 Processing redemption..."

# Process the redemption
TX_HASH=$(cast send $ASSET_ADDRESS \
  "processRedemption(address,uint256,address,bytes)" \
  $INVESTOR_WALLET \
  $PENDING \
  $INVESTOR_WALLET \
  "0x" \
  --private-key $OPERATOR_PRIVATE_KEY \
  --rpc-url $MONAD_RPC_URL \
  --json | jq -r '.transactionHash')

echo "✅ Redemption processed!"
echo "Transaction: $TX_HASH"
echo ""
echo "🔗 View on explorer:"
echo "https://explorer.monad.xyz/tx/$TX_HASH"
