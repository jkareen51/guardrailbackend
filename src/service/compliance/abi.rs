use anyhow::{Context, Result};
use ethers_core::abi::Abi;

const COMPLIANCE_REGISTRY_ABI_JSON: &str = r#"
[
  {
    "type": "function",
    "name": "addToWhitelist",
    "inputs": [
      { "name": "investor", "type": "address", "internalType": "address" }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "removeFromWhitelist",
    "inputs": [
      { "name": "investor", "type": "address", "internalType": "address" }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "batchAddToWhitelist",
    "inputs": [
      { "name": "investors", "type": "address[]", "internalType": "address[]" }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "isWhitelisted",
    "inputs": [
      { "name": "investor", "type": "address", "internalType": "address" }
    ],
    "outputs": [
      { "name": "isWhitelisted", "type": "bool", "internalType": "bool" }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "setInvestorStatus",
    "inputs": [
      { "name": "investor", "type": "address", "internalType": "address" },
      { "name": "investorIsAccredited", "type": "bool", "internalType": "bool" }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "isAccredited",
    "inputs": [
      { "name": "investor", "type": "address", "internalType": "address" }
    ],
    "outputs": [
      { "name": "isAccredited", "type": "bool", "internalType": "bool" }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "batchSetInvestorData",
    "inputs": [
      { "name": "investors", "type": "address[]", "internalType": "address[]" },
      {
        "name": "data",
        "type": "tuple[]",
        "internalType": "struct ICompliance.InvestorData[]",
        "components": [
          { "name": "isVerified", "type": "bool", "internalType": "bool" },
          { "name": "isAccredited", "type": "bool", "internalType": "bool" },
          { "name": "isFrozen", "type": "bool", "internalType": "bool" },
          { "name": "validUntil", "type": "uint64", "internalType": "uint64" },
          { "name": "jurisdiction", "type": "bytes32", "internalType": "bytes32" },
          { "name": "externalRef", "type": "bytes32", "internalType": "bytes32" }
        ]
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "canRedeem",
    "inputs": [
      { "name": "asset", "type": "address", "internalType": "address" },
      { "name": "investor", "type": "address", "internalType": "address" },
      { "name": "amount", "type": "uint256", "internalType": "uint256" }
    ],
    "outputs": [
      { "name": "isValid", "type": "bool", "internalType": "bool" },
      { "name": "reason", "type": "bytes32", "internalType": "bytes32" }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "canSubscribe",
    "inputs": [
      { "name": "asset", "type": "address", "internalType": "address" },
      { "name": "investor", "type": "address", "internalType": "address" },
      { "name": "amount", "type": "uint256", "internalType": "uint256" },
      { "name": "resultingBalance", "type": "uint256", "internalType": "uint256" }
    ],
    "outputs": [
      { "name": "isValid", "type": "bool", "internalType": "bool" },
      { "name": "reason", "type": "bytes32", "internalType": "bytes32" }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "canTransfer",
    "inputs": [
      { "name": "asset", "type": "address", "internalType": "address" },
      { "name": "from", "type": "address", "internalType": "address" },
      { "name": "to", "type": "address", "internalType": "address" },
      { "name": "amount", "type": "uint256", "internalType": "uint256" },
      { "name": "receivingBalance", "type": "uint256", "internalType": "uint256" }
    ],
    "outputs": [
      { "name": "isValid", "type": "bool", "internalType": "bool" },
      { "name": "reason", "type": "bytes32", "internalType": "bytes32" }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "getAssetRules",
    "inputs": [
      { "name": "asset", "type": "address", "internalType": "address" }
    ],
    "outputs": [
      {
        "name": "rules",
        "type": "tuple",
        "internalType": "struct ICompliance.AssetRules",
        "components": [
          { "name": "transfersEnabled", "type": "bool", "internalType": "bool" },
          { "name": "subscriptionsEnabled", "type": "bool", "internalType": "bool" },
          { "name": "redemptionsEnabled", "type": "bool", "internalType": "bool" },
          { "name": "requiresAccreditation", "type": "bool", "internalType": "bool" },
          { "name": "minInvestment", "type": "uint256", "internalType": "uint256" },
          { "name": "maxInvestorBalance", "type": "uint256", "internalType": "uint256" }
        ]
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "getInvestorData",
    "inputs": [
      { "name": "investor", "type": "address", "internalType": "address" }
    ],
    "outputs": [
      {
        "name": "data",
        "type": "tuple",
        "internalType": "struct ICompliance.InvestorData",
        "components": [
          { "name": "isVerified", "type": "bool", "internalType": "bool" },
          { "name": "isAccredited", "type": "bool", "internalType": "bool" },
          { "name": "isFrozen", "type": "bool", "internalType": "bool" },
          { "name": "validUntil", "type": "uint64", "internalType": "uint64" },
          { "name": "jurisdiction", "type": "bytes32", "internalType": "bytes32" },
          { "name": "externalRef", "type": "bytes32", "internalType": "bytes32" }
        ]
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "isJurisdictionRestricted",
    "inputs": [
      { "name": "asset", "type": "address", "internalType": "address" },
      { "name": "jurisdiction", "type": "bytes32", "internalType": "bytes32" }
    ],
    "outputs": [
      { "name": "restricted", "type": "bool", "internalType": "bool" }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "getAccessControl",
    "inputs": [],
    "outputs": [
      { "name": "", "type": "address", "internalType": "address" }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "setAssetRules",
    "inputs": [
      { "name": "asset", "type": "address", "internalType": "address" },
      {
        "name": "rules",
        "type": "tuple",
        "internalType": "struct ICompliance.AssetRules",
        "components": [
          { "name": "transfersEnabled", "type": "bool", "internalType": "bool" },
          { "name": "subscriptionsEnabled", "type": "bool", "internalType": "bool" },
          { "name": "redemptionsEnabled", "type": "bool", "internalType": "bool" },
          { "name": "requiresAccreditation", "type": "bool", "internalType": "bool" },
          { "name": "minInvestment", "type": "uint256", "internalType": "uint256" },
          { "name": "maxInvestorBalance", "type": "uint256", "internalType": "uint256" }
        ]
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "setInvestorData",
    "inputs": [
      { "name": "investor", "type": "address", "internalType": "address" },
      {
        "name": "data",
        "type": "tuple",
        "internalType": "struct ICompliance.InvestorData",
        "components": [
          { "name": "isVerified", "type": "bool", "internalType": "bool" },
          { "name": "isAccredited", "type": "bool", "internalType": "bool" },
          { "name": "isFrozen", "type": "bool", "internalType": "bool" },
          { "name": "validUntil", "type": "uint64", "internalType": "uint64" },
          { "name": "jurisdiction", "type": "bytes32", "internalType": "bytes32" },
          { "name": "externalRef", "type": "bytes32", "internalType": "bytes32" }
        ]
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "setJurisdictionRestriction",
    "inputs": [
      { "name": "asset", "type": "address", "internalType": "address" },
      { "name": "jurisdiction", "type": "bytes32", "internalType": "bytes32" },
      { "name": "restricted", "type": "bool", "internalType": "bool" }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "setAccessControl",
    "inputs": [
      { "name": "newAccessControl", "type": "address", "internalType": "address" }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  }
]
"#;

pub fn compliance_registry_abi() -> Result<Abi> {
    serde_json::from_str(COMPLIANCE_REGISTRY_ABI_JSON)
        .context("failed to decode compliance registry ABI JSON")
}
