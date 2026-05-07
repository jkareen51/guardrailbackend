use anyhow::{Context, Result};
use ethers_core::abi::Abi;

const ASSET_FACTORY_ABI_JSON: &str = r#"
[
  { "type": "function", "name": "accessControl", "inputs": [], "outputs": [{ "name": "", "type": "address", "internalType": "address" }], "stateMutability": "view" },
  { "type": "function", "name": "complianceRegistry", "inputs": [], "outputs": [{ "name": "", "type": "address", "internalType": "address" }], "stateMutability": "view" },
  { "type": "function", "name": "treasury", "inputs": [], "outputs": [{ "name": "", "type": "address", "internalType": "address" }], "stateMutability": "view" },
  { "type": "function", "name": "paused", "inputs": [], "outputs": [{ "name": "isPaused", "type": "bool", "internalType": "bool" }], "stateMutability": "view" },
  { "type": "function", "name": "createAsset", "inputs": [
    { "name": "proposalId", "type": "uint256", "internalType": "uint256" },
    { "name": "assetTypeId", "type": "bytes32", "internalType": "bytes32" },
    { "name": "name", "type": "string", "internalType": "string" },
    { "name": "symbol", "type": "string", "internalType": "string" },
    { "name": "maxSupply", "type": "uint256", "internalType": "uint256" },
    { "name": "data", "type": "bytes", "internalType": "bytes" }
  ], "outputs": [{ "name": "tokenAddress", "type": "address", "internalType": "address" }], "stateMutability": "nonpayable" },
  { "type": "function", "name": "registerAssetType", "inputs": [
    { "name": "assetTypeId", "type": "bytes32", "internalType": "bytes32" },
    { "name": "assetTypeName", "type": "string", "internalType": "string" },
    { "name": "implementation", "type": "address", "internalType": "address" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "unregisterAssetType", "inputs": [
    { "name": "assetTypeId", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "pauseFactory", "inputs": [], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "unpauseFactory", "inputs": [], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "setComplianceRegistry", "inputs": [
    { "name": "newRegistry", "type": "address", "internalType": "address" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "setTreasury", "inputs": [
    { "name": "newTreasury", "type": "address", "internalType": "address" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "isAssetTypeRegistered", "inputs": [
    { "name": "assetTypeId", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [{ "name": "", "type": "bool", "internalType": "bool" }], "stateMutability": "view" },
  { "type": "function", "name": "getAssetTypeImplementation", "inputs": [
    { "name": "assetTypeId", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [{ "name": "", "type": "address", "internalType": "address" }], "stateMutability": "view" },
  { "type": "function", "name": "getAssetTypeName", "inputs": [
    { "name": "assetTypeId", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [{ "name": "", "type": "string", "internalType": "string" }], "stateMutability": "view" },
  { "type": "function", "name": "getAllRegisteredAssetTypes", "inputs": [], "outputs": [
    { "name": "", "type": "bytes32[]", "internalType": "bytes32[]" }
  ], "stateMutability": "view" },
  { "type": "function", "name": "getAssetAddress", "inputs": [
    { "name": "proposalId", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [{ "name": "tokenAddress", "type": "address", "internalType": "address" }], "stateMutability": "view" },
  { "type": "function", "name": "getAllAssets", "inputs": [], "outputs": [
    { "name": "assets", "type": "address[]", "internalType": "address[]" }
  ], "stateMutability": "view" },
  { "type": "function", "name": "getAssetsByType", "inputs": [
    { "name": "assetTypeId", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [{ "name": "assets", "type": "address[]", "internalType": "address[]" }], "stateMutability": "view" },
  { "type": "function", "name": "getTotalAssetsCreated", "inputs": [], "outputs": [
    { "name": "count", "type": "uint256", "internalType": "uint256" }
  ], "stateMutability": "view" }
]
"#;

const BASE_ASSET_TOKEN_ABI_JSON: &str = r#"
[
  { "type": "function", "name": "issue", "inputs": [
    { "name": "to", "type": "address", "internalType": "address" },
    { "name": "amount", "type": "uint256", "internalType": "uint256" },
    { "name": "data", "type": "bytes", "internalType": "bytes" }
  ], "outputs": [{ "name": "success", "type": "bool", "internalType": "bool" }], "stateMutability": "nonpayable" },
  { "type": "function", "name": "burn", "inputs": [
    { "name": "from", "type": "address", "internalType": "address" },
    { "name": "amount", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [{ "name": "success", "type": "bool", "internalType": "bool" }], "stateMutability": "nonpayable" },
  { "type": "function", "name": "claimYield", "inputs": [
    { "name": "recipient", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "claimedAmount", "type": "uint256", "internalType": "uint256" }], "stateMutability": "nonpayable" },
  { "type": "function", "name": "redeem", "inputs": [
    { "name": "amount", "type": "uint256", "internalType": "uint256" },
    { "name": "data", "type": "bytes", "internalType": "bytes" }
  ], "outputs": [{ "name": "valueReturned", "type": "uint256", "internalType": "uint256" }], "stateMutability": "nonpayable" },
  { "type": "function", "name": "processRedemption", "inputs": [
    { "name": "investor", "type": "address", "internalType": "address" },
    { "name": "amount", "type": "uint256", "internalType": "uint256" },
    { "name": "recipient", "type": "address", "internalType": "address" },
    { "name": "data", "type": "bytes", "internalType": "bytes" }
  ], "outputs": [{ "name": "valueReturned", "type": "uint256", "internalType": "uint256" }], "stateMutability": "nonpayable" },
  { "type": "function", "name": "cancelRedemption", "inputs": [
    { "name": "amount", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [{ "name": "cancelled", "type": "bool", "internalType": "bool" }], "stateMutability": "nonpayable" },
  { "type": "function", "name": "previewPurchase", "inputs": [
    { "name": "tokenAmount", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [{ "name": "cost", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "previewRedemption", "inputs": [
    { "name": "tokenAmount", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [{ "name": "valueReturned", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "claimableYieldOf", "inputs": [
    { "name": "account", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "amount", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "accumulativeYieldOf", "inputs": [
    { "name": "account", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "amount", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "pendingRedemptionOf", "inputs": [
    { "name": "account", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "amount", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "lockedBalanceOf", "inputs": [
    { "name": "account", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "amount", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "canTransfer", "inputs": [
    { "name": "to", "type": "address", "internalType": "address" },
    { "name": "amount", "type": "uint256", "internalType": "uint256" },
    { "name": "data", "type": "bytes", "internalType": "bytes" }
  ], "outputs": [
    { "name": "statusCode", "type": "bytes1", "internalType": "bytes1" },
    { "name": "reasonCode", "type": "bytes32", "internalType": "bytes32" }
  ], "stateMutability": "view" },
  { "type": "function", "name": "getAssetState", "inputs": [], "outputs": [
    { "name": "state", "type": "uint8", "internalType": "enum IAssetToken.AssetState" }
  ], "stateMutability": "view" },
  { "type": "function", "name": "getAssetTypeId", "inputs": [], "outputs": [
    { "name": "assetTypeId", "type": "bytes32", "internalType": "bytes32" }
  ], "stateMutability": "view" },
  { "type": "function", "name": "getProposalId", "inputs": [], "outputs": [
    { "name": "proposalId", "type": "uint256", "internalType": "uint256" }
  ], "stateMutability": "view" },
  { "type": "function", "name": "isControllable", "inputs": [], "outputs": [
    { "name": "isControllable", "type": "bool", "internalType": "bool" }
  ], "stateMutability": "view" },
  { "type": "function", "name": "pricePerToken", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "redemptionPricePerToken", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "maxSupply", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "treasury", "inputs": [], "outputs": [{ "name": "", "type": "address", "internalType": "address" }], "stateMutability": "view" },
  { "type": "function", "name": "paymentToken", "inputs": [], "outputs": [{ "name": "", "type": "address", "internalType": "address" }], "stateMutability": "view" },
  { "type": "function", "name": "metadataHash", "inputs": [], "outputs": [{ "name": "", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" },
  { "type": "function", "name": "selfServicePurchaseEnabled", "inputs": [], "outputs": [{ "name": "", "type": "bool", "internalType": "bool" }], "stateMutability": "view" },
  { "type": "function", "name": "complianceRegistry", "inputs": [], "outputs": [{ "name": "", "type": "address", "internalType": "contract ICompliance" }], "stateMutability": "view" },
  { "type": "function", "name": "holderCount", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "totalPendingRedemptions", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "name", "inputs": [], "outputs": [{ "name": "", "type": "string", "internalType": "string" }], "stateMutability": "view" },
  { "type": "function", "name": "symbol", "inputs": [], "outputs": [{ "name": "", "type": "string", "internalType": "string" }], "stateMutability": "view" },
  { "type": "function", "name": "totalSupply", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "balanceOf", "inputs": [
    { "name": "account", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "setAssetState", "inputs": [
    { "name": "newState", "type": "uint8", "internalType": "enum IAssetToken.AssetState" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "setComplianceRegistry", "inputs": [
    { "name": "registry", "type": "address", "internalType": "address" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "setTreasury", "inputs": [
    { "name": "treasuryAddress", "type": "address", "internalType": "address" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "setPricePerToken", "inputs": [
    { "name": "newPrice", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "setRedemptionPricePerToken", "inputs": [
    { "name": "newPrice", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "setPricing", "inputs": [
    { "name": "newSubscriptionPrice", "type": "uint256", "internalType": "uint256" },
    { "name": "newRedemptionPrice", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "setSelfServicePurchaseEnabled", "inputs": [
    { "name": "enabled", "type": "bool", "internalType": "bool" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "setMetadataHash", "inputs": [
    { "name": "newMetadataHash", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "disableController", "inputs": [], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "controllerTransfer", "inputs": [
    { "name": "from", "type": "address", "internalType": "address" },
    { "name": "to", "type": "address", "internalType": "address" },
    { "name": "amount", "type": "uint256", "internalType": "uint256" },
    { "name": "data", "type": "bytes", "internalType": "bytes" },
    { "name": "operatorData", "type": "bytes", "internalType": "bytes" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "purchase", "inputs": [
    { "name": "tokenAmount", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [], "stateMutability": "nonpayable" }
]
"#;

const ERC20_ABI_JSON: &str = r#"
[
  { "type": "function", "name": "approve", "inputs": [
    { "name": "spender", "type": "address", "internalType": "address" },
    { "name": "amount", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [{ "name": "", "type": "bool", "internalType": "bool" }], "stateMutability": "nonpayable" },
  { "type": "function", "name": "allowance", "inputs": [
    { "name": "owner", "type": "address", "internalType": "address" },
    { "name": "spender", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "balanceOf", "inputs": [
    { "name": "account", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "symbol", "inputs": [], "outputs": [{ "name": "", "type": "string", "internalType": "string" }], "stateMutability": "view" },
  { "type": "function", "name": "decimals", "inputs": [], "outputs": [{ "name": "", "type": "uint8", "internalType": "uint8" }], "stateMutability": "view" }
]
"#;

pub fn asset_factory_abi() -> Result<Abi> {
    serde_json::from_str(ASSET_FACTORY_ABI_JSON).context("failed to decode asset factory ABI")
}

pub fn base_asset_token_abi() -> Result<Abi> {
    serde_json::from_str(BASE_ASSET_TOKEN_ABI_JSON).context("failed to decode base asset token ABI")
}

pub fn erc20_abi() -> Result<Abi> {
    serde_json::from_str(ERC20_ABI_JSON).context("failed to decode ERC20 ABI")
}
