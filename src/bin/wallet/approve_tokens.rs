//! Token approval utility for Polymarket CLOB trading
//!
//! This script sets the required token allowances for trading on Polymarket.
//! You must approve:
//!
//! 1. **CTF Exchange** (0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E) - Standard market trading
//! 2. **Neg Risk CTF Exchange** (0xC5d563A36AE78145C45a50134d48A1215220f80a) - Neg-risk market trading
//!
//! Each contract needs two approvals:
//! - ERC-20 approval for USDC (collateral token)
//! - ERC-1155 approval for Conditional Tokens (outcome tokens)
//!
//! Usage:
//!   cargo run --release --bin approve_tokens
//!
//! Dry run (check current approvals without executing):
//!   cargo run --release --bin approve_tokens -- --dry-run

use anyhow::{Result, anyhow};
use dotenvy::dotenv;
use std::env;
use std::str::FromStr;
use std::time::Duration;
use alloy::primitives::{Address, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use tokio::time::sleep;

// Contract addresses
const USDC_ADDRESS: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
const CONDITIONAL_TOKENS: &str = "0x4d97dcd97ec945f40cf65f87097ace5ea0476045";
const CTF_EXCHANGE: &str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";
const NEG_RISK_EXCHANGE: &str = "0xC5d563A36AE78145C45a50134d48A1215220f80a";

const DEFAULT_RPC_URL: &str = "https://polygon-rpc.com";
const TRANSACTION_DELAY_SECS: u64 = 3; // Delay between transactions to avoid rate limits
const MAX_RETRIES: u32 = 5;
const INITIAL_RETRY_DELAY_SECS: u64 = 10;

// Define ERC20 and ERC1155 interfaces
sol! {
    #[sol(rpc)]
    interface IERC20 {
        function approve(address spender, uint256 value) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
    }

    #[sol(rpc)]
    interface IERC1155 {
        function setApprovalForAll(address operator, bool approved) external;
        function isApprovedForAll(address account, address operator) external view returns (bool);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let args: Vec<String> = env::args().collect();
    let dry_run = args.iter().any(|arg| arg == "--dry-run");

    println!("🔐 Polymarket Token Approval Utility");
    println!("=====================================\n");

    if dry_run {
        println!("⚠️  DRY RUN MODE - No transactions will be executed\n");
    }

    // Load private key from environment
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set. Add it to your .env file."))?;

    // Parse addresses
    let usdc_addr = Address::from_str(USDC_ADDRESS)?;
    let ctf_addr = Address::from_str(CONDITIONAL_TOKENS)?;
    let ctf_exchange = Address::from_str(CTF_EXCHANGE)?;
    let neg_risk_exchange = Address::from_str(NEG_RISK_EXCHANGE)?;

    // Setup signer
    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    
    // Get RPC URL - prefer Alchemy if available (better rate limits)
    let rpc_url = if let Ok(key) = env::var("ALCHEMY_API_KEY") {
        let key = key.trim();
        if !key.is_empty() && key != "your_alchemy_api_key_here" {
            format!("https://polygon-mainnet.g.alchemy.com/v2/{}", key)
        } else {
            DEFAULT_RPC_URL.to_string()
        }
    } else {
        DEFAULT_RPC_URL.to_string()
    };
    
    println!("🌐 Using RPC: {}\n", if rpc_url.contains("alchemy") { "Alchemy (recommended)" } else { "Public RPC (may have rate limits)" });
    
    // Load funder address (Gnosis Safe) if provided, otherwise use signer address
    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| signer.address());
    
    let wallet_address = signer.address();
    println!("📝 Signer Wallet: {}", wallet_address);
    println!("🏦 Funder Address (Gnosis Safe): {}", funder_address);
    
    if funder_address != wallet_address {
        println!("   ℹ️  Approvals will be set for the Gnosis Safe address\n");
    } else {
        println!("\n");
    }
    
    // Setup provider with wallet
    let provider = ProviderBuilder::new()
        .wallet(signer.clone())
        .connect_http(rpc_url.parse()?);

    // Create contract instances
    let usdc = IERC20::new(usdc_addr, provider.clone());
    let ctf = IERC1155::new(ctf_addr, provider.clone());

    // Check current balances and allowances
    println!("📊 Checking current status...\n");

    // Check balances and allowances for the funder address (Gnosis Safe)
    let usdc_balance = usdc.balanceOf(funder_address).call().await?;
    println!("   USDC Balance (funder): {} USDC", format_units(usdc_balance, 6));

    let ctf_allowance = usdc.allowance(funder_address, ctf_exchange).call().await?;
    let neg_risk_allowance = usdc.allowance(funder_address, neg_risk_exchange).call().await?;
    
    println!("   USDC Allowance (CTF Exchange): {} USDC", format_units(ctf_allowance, 6));
    println!("   USDC Allowance (Neg Risk Exchange): {} USDC", format_units(neg_risk_allowance, 6));

    let ctf_approved = ctf.isApprovedForAll(funder_address, ctf_exchange).call().await?;
    let neg_risk_approved = ctf.isApprovedForAll(funder_address, neg_risk_exchange).call().await?;
    
    println!("   CTF Approved (CTF Exchange): {}", ctf_approved);
    println!("   CTF Approved (Neg Risk Exchange): {}\n", neg_risk_approved);

    if dry_run {
        println!("✅ Dry run complete. Run without --dry-run to execute approvals.");
        return Ok(());
    }

    // Check if approvals are needed
    let needs_usdc_ctf = ctf_allowance < U256::from(1000_000_000u64); // Less than 1000 USDC
    let needs_usdc_neg = neg_risk_allowance < U256::from(1000_000_000u64);
    let needs_ctf_ctf = !ctf_approved;
    let needs_ctf_neg = !neg_risk_approved;

    if !needs_usdc_ctf && !needs_usdc_neg && !needs_ctf_ctf && !needs_ctf_neg {
        println!("✅ All approvals are already set. No action needed.");
        return Ok(());
    }

    println!("🔧 Setting approvals...\n");

    // Helper function to retry on rate limit errors
    async fn retry_on_rate_limit<F, Fut>(mut f: F, description: &str) -> Result<()>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<alloy::primitives::FixedBytes<32>, anyhow::Error>>,
    {
        let mut delay = INITIAL_RETRY_DELAY_SECS;
        for attempt in 1..=MAX_RETRIES {
            match f().await {
                Ok(tx_hash) => {
                    println!("   ✅ {}: {:?}\n", description, tx_hash);
                    return Ok(());
                }
                Err(e) => {
                    let error_str = e.to_string();
                    let is_rate_limit = error_str.contains("rate limit") || 
                                       error_str.contains("Too many requests") ||
                                       error_str.contains("-32090");
                    
                    if is_rate_limit && attempt < MAX_RETRIES {
                        println!("   ⏳ Rate limit hit, waiting {}s before retry {}/{}...", delay, attempt + 1, MAX_RETRIES);
                        sleep(Duration::from_secs(delay)).await;
                        delay = (delay * 2).min(60); // Exponential backoff, max 60s
                    } else {
                        println!("   ❌ {} failed: {}\n", description, e);
                        if attempt < MAX_RETRIES && is_rate_limit {
                            println!("   ⏳ Retrying in {}s...", delay);
                            sleep(Duration::from_secs(delay)).await;
                            delay = (delay * 2).min(60);
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }
        Err(anyhow!("Failed after {} attempts", MAX_RETRIES))
    }

    // For Gnosis Safe, approvals must be done through Safe transactions
    if funder_address != wallet_address && !dry_run {
        println!("⚠️  IMPORTANT: Funder is a Gnosis Safe address ({})", funder_address);
        println!("   Direct approvals from private key won't work for Gnosis Safe.");
        println!("   You need to approve through your Gnosis Safe interface:\n");
        println!("   📝 Manual Approval Steps:");
        println!("   1. Go to https://app.safe.global/");
        println!("   2. Connect and select your Safe: {}", funder_address);
        println!("   3. Go to 'Apps' → Search 'Transaction Builder' or use Polymarket app");
        println!("   4. Create transactions to approve:\n");
        
        if needs_usdc_ctf || needs_usdc_neg {
            println!("   For USDC Approval:");
            if needs_usdc_ctf {
                println!("     - Contract: {}", USDC_ADDRESS);
                println!("     - Method: approve(address,uint256)");
                println!("     - Spender: {} (CTF Exchange)", CTF_EXCHANGE);
                println!("     - Amount: Max");
            }
            if needs_usdc_neg {
                println!("     - Contract: {}", USDC_ADDRESS);
                println!("     - Method: approve(address,uint256)");
                println!("     - Spender: {} (Neg Risk Exchange)", NEG_RISK_EXCHANGE);
                println!("     - Amount: Max");
            }
            println!();
        }
        
        if needs_ctf_ctf || needs_ctf_neg {
            println!("   For Conditional Tokens Approval:");
            if needs_ctf_ctf {
                println!("     - Contract: {}", CONDITIONAL_TOKENS);
                println!("     - Method: setApprovalForAll(address,bool)");
                println!("     - Operator: {} (CTF Exchange)", CTF_EXCHANGE);
                println!("     - Approved: true");
            }
            if needs_ctf_neg {
                println!("     - Contract: {}", CONDITIONAL_TOKENS);
                println!("     - Method: setApprovalForAll(address,bool)");
                println!("     - Operator: {} (Neg Risk Exchange)", NEG_RISK_EXCHANGE);
                println!("     - Approved: true");
            }
            println!();
        }
        
        println!("   5. Sign and execute the Safe transaction(s)\n");
        println!("   ❌ Cannot auto-approve for Gnosis Safe. Please approve manually as shown above.");
        return Ok(());
    }

    // Regular EOA wallet - can approve directly
    // Approve USDC for CTF Exchange
    if needs_usdc_ctf {
        println!("   Approving USDC for CTF Exchange...");
        let usdc_clone = usdc.clone();
        let ctf_exchange_clone = ctf_exchange;
        retry_on_rate_limit(
            move || {
                let usdc = usdc_clone.clone();
                let ctf_exchange = ctf_exchange_clone;
                async move {
                    let pending_tx = usdc.approve(ctf_exchange, U256::MAX).send().await?;
                    let receipt = pending_tx.get_receipt().await?;
                    Ok(receipt.transaction_hash)
                }
            },
            "USDC approved for CTF Exchange"
        ).await.ok();
        sleep(Duration::from_secs(TRANSACTION_DELAY_SECS)).await;
    } else {
        println!("   ⏭️  USDC already approved for CTF Exchange\n");
    }

    // Approve USDC for Neg Risk Exchange
    if needs_usdc_neg {
        println!("   Approving USDC for Neg Risk Exchange...");
        let usdc_clone = usdc.clone();
        let neg_risk_exchange_clone = neg_risk_exchange;
        retry_on_rate_limit(
            move || {
                let usdc = usdc_clone.clone();
                let neg_risk_exchange = neg_risk_exchange_clone;
                async move {
                    let pending_tx = usdc.approve(neg_risk_exchange, U256::MAX).send().await?;
                    let receipt = pending_tx.get_receipt().await?;
                    Ok(receipt.transaction_hash)
                }
            },
            "USDC approved for Neg Risk Exchange"
        ).await.ok();
        sleep(Duration::from_secs(TRANSACTION_DELAY_SECS)).await;
    } else {
        println!("   ⏭️  USDC already approved for Neg Risk Exchange\n");
    }

    // Approve Conditional Tokens for CTF Exchange
    if needs_ctf_ctf {
        println!("   Approving Conditional Tokens for CTF Exchange...");
        let ctf_clone = ctf.clone();
        let ctf_exchange_clone = ctf_exchange;
        retry_on_rate_limit(
            move || {
                let ctf = ctf_clone.clone();
                let ctf_exchange = ctf_exchange_clone;
                async move {
                    let pending_tx = ctf.setApprovalForAll(ctf_exchange, true).send().await?;
                    let receipt = pending_tx.get_receipt().await?;
                    Ok(receipt.transaction_hash)
                }
            },
            "Conditional Tokens approved for CTF Exchange"
        ).await.ok();
        sleep(Duration::from_secs(TRANSACTION_DELAY_SECS)).await;
    } else {
        println!("   ⏭️  Conditional Tokens already approved for CTF Exchange\n");
    }

    // Approve Conditional Tokens for Neg Risk Exchange
    if needs_ctf_neg {
        println!("   Approving Conditional Tokens for Neg Risk Exchange...");
        let ctf_clone = ctf.clone();
        let neg_risk_exchange_clone = neg_risk_exchange;
        retry_on_rate_limit(
            move || {
                let ctf = ctf_clone.clone();
                let neg_risk_exchange = neg_risk_exchange_clone;
                async move {
                    let pending_tx = ctf.setApprovalForAll(neg_risk_exchange, true).send().await?;
                    let receipt = pending_tx.get_receipt().await?;
                    Ok(receipt.transaction_hash)
                }
            },
            "Conditional Tokens approved for Neg Risk Exchange"
        ).await.ok();
    } else {
        println!("   ⏭️  Conditional Tokens already approved for Neg Risk Exchange\n");
    }

    // Verify approvals
    println!("🔍 Verifying approvals...\n");

    let ctf_allowance_after = usdc.allowance(funder_address, ctf_exchange).call().await?;
    let neg_risk_allowance_after = usdc.allowance(funder_address, neg_risk_exchange).call().await?;
    let ctf_approved_after = ctf.isApprovedForAll(funder_address, ctf_exchange).call().await?;
    let neg_risk_approved_after = ctf.isApprovedForAll(funder_address, neg_risk_exchange).call().await?;

    println!("   USDC Allowance (CTF Exchange): {} USDC", format_units(ctf_allowance_after, 6));
    println!("   USDC Allowance (Neg Risk Exchange): {} USDC", format_units(neg_risk_allowance_after, 6));
    println!("   CTF Approved (CTF Exchange): {}", ctf_approved_after);
    println!("   CTF Approved (Neg Risk Exchange): {}\n", neg_risk_approved_after);

    let all_approved = ctf_allowance_after >= U256::from(1000_000_000u64) &&
                       neg_risk_allowance_after >= U256::from(1000_000_000u64) &&
                       ctf_approved_after &&
                       neg_risk_approved_after;

    if all_approved {
        println!("✅ All approvals verified successfully!");
        println!("\n🚀 You can now trade on Polymarket!");
    } else {
        println!("⚠️  Some approvals may not be complete. Please check the status above.");
    }

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
