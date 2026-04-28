use anyhow::{Context, Result};
use ethers_core::abi::Abi;

const TREASURY_ABI_JSON: &str = r#"
[
  { "type": "function", "name": "paymentToken", "inputs": [], "outputs": [{ "name": "", "type": "address", "internalType": "address" }], "stateMutability": "view" },
  { "type": "function", "name": "accessControl", "inputs": [], "outputs": [{ "name": "", "type": "address", "internalType": "address" }], "stateMutability": "view" },
  { "type": "function", "name": "paused", "inputs": [], "outputs": [{ "name": "", "type": "bool", "internalType": "bool" }], "stateMutability": "view" },
  { "type": "function", "name": "totalTrackedBalance", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "totalReservedYield", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "depositAssetLiquidity", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" },
    { "name": "amount", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "releaseCapital", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" },
    { "name": "amount", "type": "uint256", "internalType": "uint256" },
    { "name": "to", "type": "address", "internalType": "address" },
    { "name": "referenceId", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "depositYield", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" },
    { "name": "amount", "type": "uint256", "internalType": "uint256" },
    { "name": "data", "type": "bytes", "internalType": "bytes" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "emergencyWithdraw", "inputs": [
    { "name": "token", "type": "address", "internalType": "address" },
    { "name": "amount", "type": "uint256", "internalType": "uint256" },
    { "name": "to", "type": "address", "internalType": "address" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "getBalance", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "getReservedYield", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "getAvailableLiquidity", "inputs": [
    { "name": "asset", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "pause", "inputs": [], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "unpause", "inputs": [], "outputs": [], "stateMutability": "nonpayable" }
]
"#;

pub fn treasury_abi() -> Result<Abi> {
    serde_json::from_str(TREASURY_ABI_JSON).context("failed to decode treasury ABI")
}
