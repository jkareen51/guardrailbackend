use anyhow::{Context, Result};
use ethers_core::abi::Abi;

const ORACLE_BRIDGE_ABI_JSON: &str = r#"
[
  { "type": "function", "name": "trustedOracles", "inputs": [
    { "name": "oracle", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "", "type": "bool", "internalType": "bool" }], "stateMutability": "view" },
  { "type": "function", "name": "setTrustedOracle", "inputs": [
    { "name": "oracle", "type": "address", "internalType": "address" },
    { "name": "trusted", "type": "bool", "internalType": "bool" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "submitValuation", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" },
    { "name": "assetValue", "type": "uint256", "internalType": "uint256" },
    { "name": "navPerToken", "type": "uint256", "internalType": "uint256" },
    { "name": "referenceId", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "submitValuationAndSyncPricing", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" },
    { "name": "assetValue", "type": "uint256", "internalType": "uint256" },
    { "name": "navPerToken", "type": "uint256", "internalType": "uint256" },
    { "name": "subscriptionPrice", "type": "uint256", "internalType": "uint256" },
    { "name": "redemptionPrice", "type": "uint256", "internalType": "uint256" },
    { "name": "referenceId", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "anchorDocument", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" },
    { "name": "documentType", "type": "bytes32", "internalType": "bytes32" },
    { "name": "documentHash", "type": "bytes32", "internalType": "bytes32" },
    { "name": "referenceId", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "getLatestValuation", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" }
  ], "outputs": [{
    "name": "valuation",
    "type": "tuple",
    "internalType": "struct IOracleDataBridge.AssetValuation",
    "components": [
      { "name": "assetValue", "type": "uint256", "internalType": "uint256" },
      { "name": "navPerToken", "type": "uint256", "internalType": "uint256" },
      { "name": "updatedAt", "type": "uint64", "internalType": "uint64" },
      { "name": "referenceId", "type": "bytes32", "internalType": "bytes32" }
    ]
  }], "stateMutability": "view" },
  { "type": "function", "name": "getDocumentHash", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" },
    { "name": "documentType", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [{ "name": "documentHash", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" }
]
"#;

pub fn oracle_bridge_abi() -> Result<Abi> {
    serde_json::from_str(ORACLE_BRIDGE_ABI_JSON).context("failed to decode oracle bridge ABI")
}
