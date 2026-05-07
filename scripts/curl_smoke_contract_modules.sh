#!/usr/bin/env bash

set -euo pipefail

if [[ -f .env ]]; then
  # shellcheck disable=SC1091
  source .env
fi

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
ASSET_ADDRESS="${ASSET_ADDRESS:-${ASSET_TOKEN_ADDRESS:-}}"
ADMIN_WALLET="${ADMIN_WALLET:-${ADMIN_WALLET_ADDRESSES%%,*}}"
INVESTOR_WALLET="${INVESTOR_WALLET:-$ADMIN_WALLET}"

if [[ -z "$ASSET_ADDRESS" ]]; then
  echo "ASSET_ADDRESS or ASSET_TOKEN_ADDRESS is required"
  exit 1
fi

print_section() {
  printf '\n== %s ==\n' "$1"
}

run() {
  local label="$1"
  shift
  printf '\n[%s]\n' "$label"
  curl -sS "$@"
  printf '\n'
}

print_section "Public"
run health "$BASE_URL/health"
run asset_factory "$BASE_URL/assets/factory"
run asset "$BASE_URL/assets/$ASSET_ADDRESS"
run asset_detail "$BASE_URL/assets/$ASSET_ADDRESS/detail"
run treasury "$BASE_URL/treasury"
run treasury_asset "$BASE_URL/treasury/assets/$ASSET_ADDRESS"
run trusted_oracle "$BASE_URL/oracle/trusted-oracles/$INVESTOR_WALLET"
run oracle_valuation "$BASE_URL/oracle/assets/$ASSET_ADDRESS/valuation"
run oracle_freshness "$BASE_URL/oracle/assets/$ASSET_ADDRESS/valuation/freshness"
run compliance_investor "$BASE_URL/compliance/investors/$INVESTOR_WALLET"
run compliance_rules "$BASE_URL/compliance/assets/$ASSET_ADDRESS/rules"
run preview_purchase \
  -X POST "$BASE_URL/assets/$ASSET_ADDRESS/preview/purchase" \
  -H 'Content-Type: application/json' \
  -d '{"token_amount":"1000000000000000000"}'
run preview_redemption \
  -X POST "$BASE_URL/assets/$ASSET_ADDRESS/preview/redemption" \
  -H 'Content-Type: application/json' \
  -d '{"token_amount":"1000000000000000000"}'
run asset_check_transfer \
  -X POST "$BASE_URL/assets/$ASSET_ADDRESS/check/transfer" \
  -H 'Content-Type: application/json' \
  -d "{\"from_wallet\":\"$INVESTOR_WALLET\",\"to_wallet\":\"$INVESTOR_WALLET\",\"amount\":\"1000000000000000000\",\"data\":\"0x\"}"
run compliance_check_subscribe \
  -X POST "$BASE_URL/compliance/check/subscribe" \
  -H 'Content-Type: application/json' \
  -d "{\"asset_address\":\"$ASSET_ADDRESS\",\"investor_wallet\":\"$INVESTOR_WALLET\",\"amount\":\"1000000000000000000\",\"resulting_balance\":\"1000000000000000000\"}"
run compliance_check_transfer \
  -X POST "$BASE_URL/compliance/check/transfer" \
  -H 'Content-Type: application/json' \
  -d "{\"asset_address\":\"$ASSET_ADDRESS\",\"from_wallet\":\"$INVESTOR_WALLET\",\"to_wallet\":\"$INVESTOR_WALLET\",\"amount\":\"1000000000000000000\",\"receiving_balance\":\"0\"}"
run compliance_check_redeem \
  -X POST "$BASE_URL/compliance/check/redeem" \
  -H 'Content-Type: application/json' \
  -d "{\"asset_address\":\"$ASSET_ADDRESS\",\"investor_wallet\":\"$INVESTOR_WALLET\",\"amount\":\"1000000000000000000\"}"

if command -v cast >/dev/null 2>&1 && command -v jq >/dev/null 2>&1 && [[ -n "${OPERATOR_PRIVATE_KEY:-}" && -n "$ADMIN_WALLET" ]]; then
  print_section "Authenticated"

  challenge_json="$(curl -sS -X POST "$BASE_URL/admin/auth/wallet/challenge" \
    -H 'Content-Type: application/json' \
    -d "{\"wallet_address\":\"$ADMIN_WALLET\"}")"
  challenge_id="$(printf '%s' "$challenge_json" | jq -r '.challenge_id')"
  challenge_message="$(printf '%s' "$challenge_json" | jq -r '.message')"
  signature="$(cast wallet sign --private-key "$OPERATOR_PRIVATE_KEY" "$challenge_message")"
  connect_json="$(curl -sS -X POST "$BASE_URL/admin/auth/wallet/connect" \
    -H 'Content-Type: application/json' \
    -d "{\"challenge_id\":\"$challenge_id\",\"signature\":\"$signature\"}")"
  token="$(printf '%s' "$connect_json" | jq -r '.token')"
  auth_header="Authorization: Bearer $token"

  run admin_me "$BASE_URL/admin/me" -H "$auth_header"
  run pending_redemptions "$BASE_URL/admin/assets/$ASSET_ADDRESS/redemptions/pending" -H "$auth_header"
  run process_redemption_disabled \
    -X POST "$BASE_URL/admin/assets/$ASSET_ADDRESS/redemptions/process" \
    -H "$auth_header" \
    -H 'Content-Type: application/json' \
    -d "{\"investor_wallet\":\"$INVESTOR_WALLET\",\"amount\":\"1\",\"recipient_wallet\":\"$INVESTOR_WALLET\",\"data\":\"0x\"}"
  run cancel_redemption_disabled \
    -X POST "$BASE_URL/assets/$ASSET_ADDRESS/redemptions/cancel" \
    -H "$auth_header" \
    -H 'Content-Type: application/json' \
    -d '{"amount":"1"}'
  run factory_compliance_alias_validation \
    -X PUT "$BASE_URL/admin/assets/factory/compliance-diamond" \
    -H "$auth_header" \
    -H 'Content-Type: application/json' \
    -d '{"compliance_diamond_address":"bad"}'
  run asset_compliance_alias_validation \
    -X PUT "$BASE_URL/admin/assets/$ASSET_ADDRESS/compliance-diamond" \
    -H "$auth_header" \
    -H 'Content-Type: application/json' \
    -d '{"compliance_diamond_address":"bad"}'
else
  print_section "Authenticated"
  echo "Skipping authenticated checks: cast, jq, ADMIN_WALLET_ADDRESSES, or OPERATOR_PRIVATE_KEY not available"
fi
