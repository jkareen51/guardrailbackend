#!/usr/bin/env bash

set -euo pipefail

# Seeds agriculture assets from docs/nigeria-agriculture-admin-asset-seeds.json by:
# 1. authenticating as an admin,
# 2. converting local images to WebP when possible,
# 3. uploading each image to /admin/uploads/images,
# 4. registering the agriculture asset type(s) from the seed file,
# 5. creating each asset with non-null catalog fields.
#
# Required:
# - IMPLEMENTATION_ADDRESS or AGRICULTURE_IMPLEMENTATION_ADDRESS
# - either ADMIN_BEARER_TOKEN or ADMIN_WALLET + OPERATOR_PRIVATE_KEY
#
# Defaults:
# - seed definitions come from docs/nigeria-agriculture-admin-asset-seeds.json
# - image assignment uses sorted files from agric/ unless IMAGE_MANIFEST_FILE is set
# - proposal IDs come from that file unless START_PROPOSAL_ID is set
# - human max supply input 21000000 is converted to raw base units with
#   MAX_SUPPLY_DECIMALS=18, producing 21000000000000000000000000
# - human NGN inputs 100.00 / 100.00 are converted to raw integer strings
#   with PRICE_SCALE_DECIMALS=2, producing 10000 / 10000.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "$ROOT_DIR/.env" ]]; then
  # shellcheck disable=SC1091
  source "$ROOT_DIR/.env"
fi

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
AGRICULTURE_DIR="${AGRICULTURE_DIR:-$ROOT_DIR/agric}"
ASSET_SEEDS_FILE="${ASSET_SEEDS_FILE:-$ROOT_DIR/docs/nigeria-agriculture-admin-asset-seeds.json}"
IMAGE_MANIFEST_FILE="${IMAGE_MANIFEST_FILE:-}"
IMAGE_SCOPE="${IMAGE_SCOPE:-agriculture}"
START_PROPOSAL_ID="${START_PROPOSAL_ID:-}"
MAX_SUPPLY_TOKENS="${MAX_SUPPLY_TOKENS:-21000000}"
MAX_SUPPLY_DECIMALS="${MAX_SUPPLY_DECIMALS:-18}"
MAX_SUPPLY_RAW="${MAX_SUPPLY_RAW:-}"
PRICE_SCALE_DECIMALS="${PRICE_SCALE_DECIMALS:-2}"
SUBSCRIPTION_PRICE_NGN="${SUBSCRIPTION_PRICE_NGN:-100.00}"
REDEMPTION_PRICE_NGN="${REDEMPTION_PRICE_NGN:-100.00}"
ADMIN_BEARER_TOKEN="${ADMIN_BEARER_TOKEN:-}"
ADMIN_WALLET="${ADMIN_WALLET:-${ADMIN_WALLET_ADDRESSES:-}}"
OPERATOR_PRIVATE_KEY="${OPERATOR_PRIVATE_KEY:-}"
IMPLEMENTATION_ADDRESS_ENV="${IMPLEMENTATION_ADDRESS:-}"
IMPLEMENTATION_ADDRESS="${AGRICULTURE_IMPLEMENTATION_ADDRESS:-${ASSET_IMPLEMENTATION_ADDRESS:-${ASSET_TOKEN_ADDRESS:-$IMPLEMENTATION_ADDRESS_ENV}}}"

HTTP_STATUS=""
TMP_DIR=""
AUTH_HEADER=""
RESPONSE_BODY=""

print_section() {
  printf "\n== %s ==\n" "$1"
}

fail() {
  printf "Error: %s\n" "$1" >&2
  exit 1
}

cleanup() {
  if [[ -n "$TMP_DIR" && -d "$TMP_DIR" ]]; then
    rm -rf "$TMP_DIR"
  fi
}

trap cleanup EXIT

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

decimal_to_scaled_int() {
  local raw_value="$1"
  local scale_digits="$2"
  local whole=""
  local frac=""
  local zeros_needed=0

  [[ "$raw_value" =~ ^[0-9]+([.][0-9]+)?$ ]] || fail "invalid decimal value: $raw_value"
  [[ "$scale_digits" =~ ^[0-9]+$ ]] || fail "invalid scale digits: $scale_digits"

  if [[ "$raw_value" == *.* ]]; then
    whole="${raw_value%%.*}"
    frac="${raw_value#*.}"
  else
    whole="$raw_value"
    frac=""
  fi

  if [[ ${#frac} -gt $scale_digits ]]; then
    frac="${frac:0:$scale_digits}"
  fi

  zeros_needed=$((scale_digits - ${#frac}))
  while [[ $zeros_needed -gt 0 ]]; do
    frac="${frac}0"
    zeros_needed=$((zeros_needed - 1))
  done

  raw_value="${whole}${frac}"
  while [[ "${#raw_value}" -gt 1 && "${raw_value:0:1}" == "0" ]]; do
    raw_value="${raw_value#0}"
  done
  if [[ -z "$raw_value" ]]; then
    raw_value="0"
  fi

  printf "%s\n" "$raw_value"
}

mime_type_for() {
  local file_path="$1"
  local mime_type=""

  if command -v file >/dev/null 2>&1; then
    mime_type="$(file --mime-type -b "$file_path" 2>/dev/null || true)"
  fi

  if [[ -n "$mime_type" ]]; then
    printf "%s\n" "$mime_type"
    return
  fi

  case "${file_path##*.}" in
    jpg|jpeg|JPG|JPEG) printf "image/jpeg\n" ;;
    png|PNG) printf "image/png\n" ;;
    webp|WEBP) printf "image/webp\n" ;;
    gif|GIF) printf "image/gif\n" ;;
    svg|SVG) printf "image/svg+xml\n" ;;
    avif|AVIF) printf "image/avif\n" ;;
    *) fail "could not infer MIME type for $file_path" ;;
  esac
}

prepare_upload_file() {
  local source_file="$1"
  local prepared_file="$source_file"
  local extension="${source_file##*.}"
  local output_file=""

  extension="$(printf "%s" "$extension" | tr '[:upper:]' '[:lower:]')"
  if [[ "$extension" == "webp" ]]; then
    printf "%s\n" "$prepared_file"
    return
  fi

  output_file="$TMP_DIR/$(basename "${source_file%.*}").webp"

  if command -v cwebp >/dev/null 2>&1; then
    if cwebp -quiet -q 85 "$source_file" -o "$output_file" >/dev/null 2>&1; then
      printf "%s\n" "$output_file"
      return
    fi
  fi

  if command -v magick >/dev/null 2>&1; then
    if magick "$source_file" -quality 85 "$output_file" >/dev/null 2>&1; then
      printf "%s\n" "$output_file"
      return
    fi
  fi

  if command -v sips >/dev/null 2>&1; then
    if sips -s format webp "$source_file" --out "$output_file" >/dev/null 2>&1; then
      printf "%s\n" "$output_file"
      return
    fi
  fi

  printf "%s\n" "$prepared_file"
}

http_json() {
  local method="$1"
  local path="$2"
  local json_payload="${3:-}"
  local auth_header="${4:-}"
  local body_file=""
  local -a curl_args

  body_file="$TMP_DIR/http-body.json"
  : >"$body_file"

  curl_args=(curl -sS -o "$body_file" -w "%{http_code}" -X "$method" "$BASE_URL$path")
  if [[ -n "$auth_header" ]]; then
    curl_args+=(-H "$auth_header")
  fi
  if [[ -n "$json_payload" ]]; then
    curl_args+=(-H "Content-Type: application/json" -d "$json_payload")
  fi

  HTTP_STATUS="$("${curl_args[@]}")"
  RESPONSE_BODY="$(cat "$body_file")"
}

http_upload_image() {
  local file_path="$1"
  local scope="$2"
  local mime_type="$3"
  local body_file=""
  local -a curl_args

  body_file="$TMP_DIR/http-upload.json"
  : >"$body_file"

  curl_args=(
    curl -sS -o "$body_file" -w "%{http_code}"
    -X POST "$BASE_URL/admin/uploads/images"
    -H "$AUTH_HEADER"
    -F "scope=$scope"
    -F "file=@$file_path;type=$mime_type"
  )

  HTTP_STATUS="$("${curl_args[@]}")"
  RESPONSE_BODY="$(cat "$body_file")"
}

ensure_success() {
  local status="$1"
  local body="$2"
  local context="$3"

  if [[ "${status:0:1}" != "2" ]]; then
    printf "Request failed during %s (HTTP %s)\n" "$context" "$status" >&2
    printf "%s\n" "$body" >&2
    exit 1
  fi
}

authenticate_admin() {
  local challenge_payload=""
  local challenge_body=""
  local challenge_id=""
  local challenge_message=""
  local signature=""
  local connect_payload=""
  local connect_body=""
  local token=""

  if [[ -n "$ADMIN_BEARER_TOKEN" ]]; then
    AUTH_HEADER="Authorization: Bearer $ADMIN_BEARER_TOKEN"
    return
  fi

  require_cmd cast
  [[ -n "$ADMIN_WALLET" ]] || fail "ADMIN_WALLET or ADMIN_WALLET_ADDRESSES is required"
  [[ -n "$OPERATOR_PRIVATE_KEY" ]] || fail "OPERATOR_PRIVATE_KEY is required when ADMIN_BEARER_TOKEN is not set"

  challenge_payload="$(jq -n --arg wallet_address "$ADMIN_WALLET" '{wallet_address:$wallet_address}')"
  http_json POST "/admin/auth/wallet/challenge" "$challenge_payload"
  challenge_body="$RESPONSE_BODY"
  ensure_success "$HTTP_STATUS" "$challenge_body" "admin wallet challenge"

  challenge_id="$(printf "%s" "$challenge_body" | jq -r '.challenge_id // empty')"
  challenge_message="$(printf "%s" "$challenge_body" | jq -r '.message // empty')"
  [[ -n "$challenge_id" && -n "$challenge_message" ]] || fail "admin wallet challenge response missing challenge fields"

  signature="$(cast wallet sign --private-key "$OPERATOR_PRIVATE_KEY" "$challenge_message")"
  connect_payload="$(jq -n \
    --arg challenge_id "$challenge_id" \
    --arg signature "$signature" \
    '{challenge_id:$challenge_id, signature:$signature}')"
  http_json POST "/admin/auth/wallet/connect" "$connect_payload"
  connect_body="$RESPONSE_BODY"
  ensure_success "$HTTP_STATUS" "$connect_body" "admin wallet connect"

  token="$(printf "%s" "$connect_body" | jq -r '.token // empty')"
  [[ -n "$token" ]] || fail "admin wallet connect response missing token"
  AUTH_HEADER="Authorization: Bearer $token"
}

asset_exists_by_proposal() {
  local proposal_id="$1"

  http_json GET "/assets/proposals/$proposal_id"
  if [[ "$HTTP_STATUS" == "200" ]]; then
    return 0
  fi
  if [[ "$HTTP_STATUS" == "404" ]]; then
    return 1
  fi

  ensure_success "$HTTP_STATUS" "$RESPONSE_BODY" "check existing asset for proposal $proposal_id"
  return 1
}

asset_type_exists() {
  local asset_type_id="$1"
  local is_registered=""

  http_json GET "/assets/types/$asset_type_id"
  if [[ "$HTTP_STATUS" == "200" ]]; then
    is_registered="$(printf "%s" "$RESPONSE_BODY" | jq -r '.is_registered // false')"
    if [[ "$is_registered" == "true" ]]; then
      return 0
    fi
    return 1
  fi
  if [[ "$HTTP_STATUS" == "404" ]]; then
    return 1
  fi

  ensure_success "$HTTP_STATUS" "$RESPONSE_BODY" "check existing asset type $asset_type_id"
  return 1
}

register_asset_type() {
  local asset_type_id="$1"
  local asset_type_name="$2"
  local payload=""

  payload="$(jq -n \
    --arg asset_type_id "$asset_type_id" \
    --arg asset_type_name "$asset_type_name" \
    --arg implementation_address "$IMPLEMENTATION_ADDRESS" \
    '{
      asset_type_id:$asset_type_id,
      asset_type_name:$asset_type_name,
      implementation_address:$implementation_address
    }')"
  http_json POST "/admin/assets/types" "$payload" "$AUTH_HEADER"
  ensure_success "$HTTP_STATUS" "$RESPONSE_BODY" "register asset type $asset_type_id"
}

register_asset_types_from_seed_file() {
  local asset_type_json=""
  local asset_type_id=""
  local asset_type_name=""

  while IFS= read -r asset_type_json; do
    asset_type_id="$(printf "%s" "$asset_type_json" | jq -r '.payload.asset_type_id // empty')"
    asset_type_name="$(printf "%s" "$asset_type_json" | jq -r '.payload.asset_type_name // empty')"
    [[ -n "$asset_type_id" && -n "$asset_type_name" ]] || fail "seed file contains an invalid asset type entry"

    if asset_type_exists "$asset_type_id"; then
      printf "Asset type already registered: %s\n" "$asset_type_id"
    else
      printf "Registering asset type: %s\n" "$asset_type_id"
      register_asset_type "$asset_type_id" "$asset_type_name"
    fi
  done < <(jq -c '.asset_types[]' "$ASSET_SEEDS_FILE")
}

proposal_id_for_asset() {
  local asset_json="$1"
  local idx="$2"
  local proposal_id=""

  if [[ -n "$START_PROPOSAL_ID" ]]; then
    proposal_id=$((START_PROPOSAL_ID + idx))
    printf "%s\n" "$proposal_id"
    return
  fi

  proposal_id="$(printf "%s" "$asset_json" | jq -r '.create_payload.proposal_id // empty')"
  [[ "$proposal_id" =~ ^[0-9]+$ ]] || fail "asset is missing a numeric create_payload.proposal_id"
  printf "%s\n" "$proposal_id"
}

metadata_hash_for_asset() {
  local asset_json="$1"
  local idx="$2"
  local metadata_hash=""

  metadata_hash="$(printf "%s" "$asset_json" | jq -r '.create_payload.metadata_hash // empty')"
  if [[ -n "$metadata_hash" && "$metadata_hash" != "null" ]]; then
    printf "%s\n" "$metadata_hash"
    return
  fi

  printf '0x%064x\n' $((idx + 1))
}

create_asset() {
  local proposal_id="$1"
  local metadata_hash="$2"
  local image_url="$3"
  local asset_json="$4"
  local payload=""
  local featured=""
  local visible=""
  local searchable=""
  local self_service_purchase_enabled=""
  local tags_json=""
  local sources_json=""
  local market_segment=""
  local slug=""

  featured="$(printf "%s" "$asset_json" | jq '.create_payload.featured // false')"
  visible="$(printf "%s" "$asset_json" | jq '.create_payload.visible // true')"
  searchable="$(printf "%s" "$asset_json" | jq '.create_payload.searchable // true')"
  self_service_purchase_enabled="$(printf "%s" "$asset_json" | jq '.create_payload.self_service_purchase_enabled // false')"
  tags_json="$(printf "%s" "$asset_json" | jq -c '.suggested_internal_tags // []')"
  sources_json="$(printf "%s" "$asset_json" | jq -c '.sources // []')"
  market_segment="$(printf "%s" "$asset_json" | jq -r '.market_segment // .create_payload.market_segment // empty')"
  slug="$(printf "%s" "$asset_json" | jq -r '.create_payload.slug')-$proposal_id"

  payload="$(jq -n \
    --arg proposal_id "$proposal_id" \
    --arg asset_type_id "$(printf "%s" "$asset_json" | jq -r '.create_payload.asset_type_id')" \
    --arg name "$(printf "%s" "$asset_json" | jq -r '.create_payload.name')" \
    --arg symbol "$(printf "%s" "$asset_json" | jq -r '.create_payload.symbol')" \
    --arg max_supply "$MAX_SUPPLY_RAW" \
    --arg subscription_price "$SUBSCRIPTION_PRICE_RAW" \
    --arg redemption_price "$REDEMPTION_PRICE_RAW" \
    --arg metadata_hash "$metadata_hash" \
    --arg slug "$slug" \
    --arg image_url "$image_url" \
    --arg summary "$(printf "%s" "$asset_json" | jq -r '.create_payload.summary')" \
    --arg market_segment "$market_segment" \
    --argjson suggested_internal_tags "$tags_json" \
    --argjson sources "$sources_json" \
    --argjson featured "$featured" \
    --argjson visible "$visible" \
    --argjson searchable "$searchable" \
    --argjson self_service_purchase_enabled "$self_service_purchase_enabled" \
    '{
      proposal_id:$proposal_id,
      asset_type_id:$asset_type_id,
      name:$name,
      symbol:$symbol,
      max_supply:$max_supply,
      subscription_price:$subscription_price,
      redemption_price:$redemption_price,
      self_service_purchase_enabled:$self_service_purchase_enabled,
      metadata_hash:$metadata_hash,
      slug:$slug,
      image_url:$image_url,
      summary:$summary,
      market_segment:$market_segment,
      suggested_internal_tags:$suggested_internal_tags,
      sources:$sources,
      featured:$featured,
      visible:$visible,
      searchable:$searchable
    }')"

  http_json POST "/admin/assets" "$payload" "$AUTH_HEADER"
  ensure_success "$HTTP_STATUS" "$RESPONSE_BODY" "create asset for proposal $proposal_id"
}

resolve_image_path_for_asset() {
  local asset_json="$1"
  local fallback_image="$2"
  local asset_key=""
  local image_path=""

  if [[ -z "$IMAGE_MANIFEST_FILE" ]]; then
    printf "%s\n" "$fallback_image"
    return
  fi

  asset_key="$(printf "%s" "$asset_json" | jq -r '.key // empty')"
  [[ -n "$asset_key" ]] || fail "asset is missing key required by IMAGE_MANIFEST_FILE"

  image_path="$(jq -r --arg key "$asset_key" '.[$key] // empty' "$IMAGE_MANIFEST_FILE")"
  [[ -n "$image_path" ]] || fail "IMAGE_MANIFEST_FILE does not define an image for asset key $asset_key"

  if [[ "$image_path" != /* ]]; then
    image_path="$ROOT_DIR/$image_path"
  fi

  [[ -f "$image_path" ]] || fail "image file from IMAGE_MANIFEST_FILE not found: $image_path"
  printf "%s\n" "$image_path"
}

main() {
  local asset_count=0
  local idx=0
  local proposal_id=""
  local existing_body=""
  local asset_json=""
  local image_file=""
  local prepared_image=""
  local mime_type=""
  local upload_body=""
  local image_url=""
  local metadata_hash=""
  local create_body=""
  local created_count=0
  local skipped_count=0
  local created_asset_address=""
  local existing_asset_address=""
  local existing_asset_name=""
  local asset_name=""
  local image_count=0
  local unused_images=0
  local -a image_files=()

  require_cmd curl
  require_cmd jq

  if [[ "$ADMIN_WALLET" == *,* ]]; then
    ADMIN_WALLET="${ADMIN_WALLET%%,*}"
  fi

  [[ -f "$ASSET_SEEDS_FILE" ]] || fail "agriculture seed file not found: $ASSET_SEEDS_FILE"
  if [[ -n "$IMAGE_MANIFEST_FILE" ]]; then
    [[ -f "$IMAGE_MANIFEST_FILE" ]] || fail "IMAGE_MANIFEST_FILE not found: $IMAGE_MANIFEST_FILE"
  else
    [[ -d "$AGRICULTURE_DIR" ]] || fail "agriculture image directory not found: $AGRICULTURE_DIR"
  fi
  [[ -n "$IMPLEMENTATION_ADDRESS" ]] || fail "IMPLEMENTATION_ADDRESS or AGRICULTURE_IMPLEMENTATION_ADDRESS is required"
  if [[ -n "$START_PROPOSAL_ID" && ! "$START_PROPOSAL_ID" =~ ^[0-9]+$ ]]; then
    fail "START_PROPOSAL_ID must be an integer when provided"
  fi

  TMP_DIR="$(mktemp -d)"
  MAX_SUPPLY_RAW="${MAX_SUPPLY_RAW:-$(decimal_to_scaled_int "$MAX_SUPPLY_TOKENS" "$MAX_SUPPLY_DECIMALS")}"
  SUBSCRIPTION_PRICE_RAW="${SUBSCRIPTION_PRICE_RAW:-$(decimal_to_scaled_int "$SUBSCRIPTION_PRICE_NGN" "$PRICE_SCALE_DECIMALS")}"
  REDEMPTION_PRICE_RAW="${REDEMPTION_PRICE_RAW:-$(decimal_to_scaled_int "$REDEMPTION_PRICE_NGN" "$PRICE_SCALE_DECIMALS")}"

  [[ "$MAX_SUPPLY_RAW" =~ ^[0-9]+$ ]] || fail "MAX_SUPPLY_RAW must be an integer string"
  [[ "$SUBSCRIPTION_PRICE_RAW" =~ ^[0-9]+$ ]] || fail "SUBSCRIPTION_PRICE_RAW must be an integer string"
  [[ "$REDEMPTION_PRICE_RAW" =~ ^[0-9]+$ ]] || fail "REDEMPTION_PRICE_RAW must be an integer string"

  asset_count="$(jq '.assets | length' "$ASSET_SEEDS_FILE")"
  [[ "$asset_count" =~ ^[0-9]+$ && "$asset_count" -gt 0 ]] || fail "seed file does not contain any assets"

  if [[ -z "$IMAGE_MANIFEST_FILE" ]]; then
    while IFS= read -r image_file; do
      image_files+=("$image_file")
    done < <(find "$AGRICULTURE_DIR" -maxdepth 1 -type f | sort)

    image_count="${#image_files[@]}"
    if [[ "$image_count" -lt "$asset_count" ]]; then
      fail "expected at least $asset_count image files in $AGRICULTURE_DIR, found $image_count"
    fi
    unused_images=$((image_count - asset_count))
  fi

  authenticate_admin

  print_section "Seed Configuration"
  printf "Base URL: %s\n" "$BASE_URL"
  printf "Seed file: %s\n" "$ASSET_SEEDS_FILE"
  if [[ -n "$IMAGE_MANIFEST_FILE" ]]; then
    printf "Image manifest: %s\n" "$IMAGE_MANIFEST_FILE"
  else
    printf "Image directory: %s\n" "$AGRICULTURE_DIR"
  fi
  printf "Image scope: %s\n" "$IMAGE_SCOPE"
  printf "Implementation address: %s\n" "$IMPLEMENTATION_ADDRESS"
  printf "Max supply tokens: %s -> raw %s\n" "$MAX_SUPPLY_TOKENS" "$MAX_SUPPLY_RAW"
  printf "Subscription price NGN: %s -> raw %s\n" "$SUBSCRIPTION_PRICE_NGN" "$SUBSCRIPTION_PRICE_RAW"
  printf "Redemption price NGN: %s -> raw %s\n" "$REDEMPTION_PRICE_NGN" "$REDEMPTION_PRICE_RAW"
  if [[ -n "$START_PROPOSAL_ID" ]]; then
    printf "Proposal IDs: sequential from %s\n" "$START_PROPOSAL_ID"
  else
    printf "Proposal IDs: from seed file\n"
  fi
  if [[ -z "$IMAGE_MANIFEST_FILE" && "$unused_images" -gt 0 ]]; then
    printf "Extra images ignored: %s\n" "$unused_images"
  fi

  print_section "Register Asset Types"
  register_asset_types_from_seed_file

  idx=0
  while [[ "$idx" -lt "$asset_count" ]]; do
    asset_json="$(jq -c ".assets[$idx]" "$ASSET_SEEDS_FILE")"
    image_file="${image_files[$idx]:-}"
    proposal_id="$(proposal_id_for_asset "$asset_json" "$idx")"
    metadata_hash="$(metadata_hash_for_asset "$asset_json" "$idx")"
    asset_name="$(printf "%s" "$asset_json" | jq -r '.create_payload.name // empty')"
    [[ -n "$asset_name" ]] || fail "asset entry $idx is missing create_payload.name"
    image_file="$(resolve_image_path_for_asset "$asset_json" "$image_file")"

    print_section "Asset $((idx + 1))/$asset_count"
    printf "Name: %s\n" "$asset_name"
    printf "Proposal ID: %s\n" "$proposal_id"
    printf "Image: %s\n" "$image_file"

    if asset_exists_by_proposal "$proposal_id"; then
      existing_body="$RESPONSE_BODY"
      existing_asset_address="$(printf "%s" "$existing_body" | jq -r '.asset_address // empty')"
      existing_asset_name="$(printf "%s" "$existing_body" | jq -r '.name // empty')"
      printf "Skipping existing asset %s at %s\n" "$existing_asset_name" "$existing_asset_address"
      skipped_count=$((skipped_count + 1))
      idx=$((idx + 1))
      continue
    fi

    prepared_image="$(prepare_upload_file "$image_file")"
    mime_type="$(mime_type_for "$prepared_image")"
    printf "Uploading image: %s (%s)\n" "$prepared_image" "$mime_type"
    http_upload_image "$prepared_image" "$IMAGE_SCOPE" "$mime_type"
    upload_body="$RESPONSE_BODY"
    ensure_success "$HTTP_STATUS" "$upload_body" "upload image for $asset_name"
    image_url="$(printf "%s" "$upload_body" | jq -r '.asset.gateway_url // empty')"
    [[ -n "$image_url" ]] || fail "upload response missing gateway_url for $asset_name"

    create_asset "$proposal_id" "$metadata_hash" "$image_url" "$asset_json"
    create_body="$RESPONSE_BODY"
    created_asset_address="$(printf "%s" "$create_body" | jq -r '.asset.asset_address // empty')"
    [[ -n "$created_asset_address" ]] || fail "create asset response missing asset address for $asset_name"

    printf "Created asset at %s\n" "$created_asset_address"
    created_count=$((created_count + 1))
    idx=$((idx + 1))
  done

  print_section "Done"
  printf "Created: %s\n" "$created_count"
  printf "Skipped existing: %s\n" "$skipped_count"
  if [[ -z "$IMAGE_MANIFEST_FILE" ]]; then
    printf "Unused images: %s\n" "$unused_images"
  fi
}

main "$@"
