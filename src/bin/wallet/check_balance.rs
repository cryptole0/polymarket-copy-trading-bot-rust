//! Check wallet balance utility
//! Run with: cargo run --release --bin check_balance
//!
//! Checks USDC and MATIC balance for the funder address

use anyhow::{Result, anyhow};
use dotenvy::dotenv;
use std::env;
use std::str::FromStr;
use alloy::primitives::{Address, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;

const USDC_ADDRESS: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
const DEFAULT_RPC_URL: &str = "https://polygon-rpc.com";

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    println!("💰 Wallet Balance Checker");
    println!("=========================\n");

    // Load funder address (Gnosis Safe) if provided, otherwise use signer address
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set. Add it to your .env file."))?;

    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;

    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| signer.address());

    let wallet_address = signer.address();
    println!("📝 Signer Wallet: {}", wallet_address);
    println!("🏦 Funder Address: {}", funder_address);
    
    if funder_address != wallet_address {
        println!("   ℹ️  Checking balance for Gnosis Safe address\n");
    } else {
        println!("\n");
    }

    // Get RPC URL
    let rpc_url = if let Ok(key) = env::var("ALCHEMY_API_KEY") {
        let key = key.trim();
        if !key.is_empty() && key != "your_alchemy_api_key_here" {
            format!("https://polygon-mainnet.g.alchemy.com/v2/{}", key)
        } else {
            DEFAULT_RPC_URL.to_string()
        }
    } else if let Ok(key) = env::var("CHAINSTACK_API_KEY") {
        let key = key.trim();
        if !key.is_empty() && key != "your_chainstack_api_key_here" {
            format!("https://polygon-mainnet.gateway.pokt.network/v1/lb/{}", key)
        } else {
            DEFAULT_RPC_URL.to_string()
        }
    } else {
        DEFAULT_RPC_URL.to_string()
    };

    println!("🌐 Using RPC: {}\n", if rpc_url.contains("alchemy") { "Alchemy" } else if rpc_url.contains("chainstack") { "Chainstack" } else { "Public RPC" });

    // Setup provider with wallet
    let provider = ProviderBuilder::new()
        .wallet(signer.clone())
        .connect_http(rpc_url.parse()?);

    // Get MATIC balance using a simple RPC call
    let client = reqwest::Client::new();
    let balance_result = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBalance",
            "params": [format!("{:#x}", funder_address), "latest"],
            "id": 1
        }))
        .send()
        .await?;
    
    let balance_json: serde_json::Value = balance_result.json().await?;
    let matic_balance_hex = balance_json["result"].as_str().unwrap_or("0x0");
    let matic_balance = U256::from_str_radix(matic_balance_hex.strip_prefix("0x").unwrap_or(matic_balance_hex), 16)?;
    let matic_balance_eth = format_units(matic_balance, 18);

    // Check USDC balance
    let usdc_addr = Address::from_str(USDC_ADDRESS)?;
    let usdc = IERC20::new(usdc_addr, provider.clone());
    let usdc_balance = usdc.balanceOf(funder_address).call().await?;
    let usdc_balance_formatted = format_units(usdc_balance, 6);

    println!("📊 Balance Summary:");
    println!("   USDC Balance: {} USDC", usdc_balance_formatted);
    println!("   MATIC Balance: {} MATIC", matic_balance_eth);

    // Warnings
    if usdc_balance < U256::from(50_000_000u64) { // Less than 50 USDC
        println!("\n⚠️  Warning: Low USDC balance (recommended: at least 50-100 USDC for trading)");
    }

    if matic_balance < U256::from(10_000_000_000_000_000u64) { // Less than 0.01 MATIC
        println!("⚠️  Warning: Low MATIC balance (recommended: at least 0.01-0.1 MATIC for gas fees)");
    }

    println!();
    Ok(())
}

fn format_units(value: U256, decimals: u32) -> String {
    let divisor = U256::from(10u64.pow(decimals));
    let whole = value / divisor;
    let remainder = value % divisor;
    
    if remainder == U256::ZERO {
        format!("{}", whole)
    } else {
        let remainder_str = format!("{:0>width$}", remainder, width = decimals as usize);
        let trimmed = remainder_str.trim_end_matches('0');
        if trimmed.is_empty() {
            format!("{}", whole)
        } else {
            format!("{}.{}", whole, trimmed)
        }
    }
}

