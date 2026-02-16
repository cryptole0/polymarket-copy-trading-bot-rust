//! Polymarket Bot CLI
//! Main entry point for all bot commands
//!
//! Usage: cargo run --release <group> <command> [args...]

use clap::{Parser, Subcommand};
use anyhow::{Result, anyhow};
use std::env;
use std::str::FromStr;
use std::fs;
use std::path::Path;
use std::io::{self, Write};
use alloy::primitives::{Address, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use serde::Deserialize;
use pm_whale_follower::settings::{Config, CopyStrategy};

#[derive(Parser)]
#[command(name = "polymarket-bot")]
#[command(about = "Polymarket Copy Trading Bot CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    group: CommandGroup,
}

#[derive(Subcommand)]
enum CommandGroup {
    /// Setup and configuration commands
    Setup {
        #[command(subcommand)]
        command: SetupCommand,
    },
    /// Main bot execution
    Main {
        #[command(subcommand)]
        command: MainCommand,
    },
    /// Wallet management commands
    Wallet {
        #[command(subcommand)]
        command: WalletCommand,
    },
    /// Position management commands
    Position {
        #[command(subcommand)]
        command: PositionCommand,
    },
    /// Trader research and analysis
    Research {
        #[command(subcommand)]
        command: ResearchCommand,
    },
    /// Simulation and backtesting
    Simulation {
        #[command(subcommand)]
        command: SimulationCommand,
    },
}

#[derive(Subcommand)]
enum SetupCommand {
    /// Interactive setup wizard for configuration
    Setup,
    /// Validate configuration, check balances, and test connectivity
    SystemStatus,
    /// Print all available commands and usage
    Help,
}

#[derive(Subcommand)]
enum MainCommand {
    /// Start the copy trading bot main loop
    Run,
}

#[derive(Subcommand)]
enum WalletCommand {
    /// Check proxy wallet (Gnosis Safe) balance and positions
    CheckProxyWallet,
    /// Compare two wallet addresses
    CheckBothWallets {
        /// First wallet address
        address1: String,
        /// Second wallet address
        address2: String,
    },
    /// View comprehensive wallet statistics
    CheckMyStats,
    /// View recent trading activity
    CheckRecentActivity,
    /// View detailed position information
    CheckPositionsDetailed,
    /// Analyze P&L discrepancies
    CheckPnlDiscrepancy,
    /// Verify token allowance
    VerifyAllowance,
    /// Check and set token allowance
    CheckAllowance,
    /// Set ERC1155 token allowance
    SetTokenAllowance,
    /// Find and analyze EOA wallet
    FindMyEoa,
    /// Find Gnosis Safe proxy wallet
    FindGnosisSafeProxy,
}

#[derive(Subcommand)]
enum PositionCommand {
    /// Manually sell a specific position
    ManualSell {
        /// Market ID
        market_id: String,
        /// Outcome (YES/NO)
        outcome: String,
        /// Amount to sell
        amount: String,
    },
    /// Sell large positions automatically
    SellLarge,
    /// Close stale/old positions
    CloseStale,
    /// Close resolved market positions
    CloseResolved,
    /// Redeem resolved positions
    RedeemResolved,
}

#[derive(Subcommand)]
enum ResearchCommand {
    /// Find best performing traders
    FindBestTraders,
    /// Find low-risk traders with good metrics
    FindLowRiskTraders,
    /// Scan and analyze top traders
    ScanBestTraders,
    /// Scan traders from active markets
    ScanFromMarkets,
}

#[derive(Subcommand)]
enum SimulationCommand {
    /// Simulate profitability for a trader
    SimulateProfitability {
        /// Trader address (optional, will prompt if not provided)
        trader_address: Option<String>,
    },
    /// Simulate profitability using old logic (legacy)
    SimulateProfitabilityOld {
        /// Trader address (optional, will prompt if not provided)
        trader_address: Option<String>,
    },
    /// Run comprehensive batch simulations
    Run {
        /// Preset: quick, standard, full, or custom
        preset: Option<String>,
    },
    /// Compare simulation results
    Compare {
        /// Mode: best [N], worst [N], stats, or detail <name>
        mode: Option<String>,
    },
    /// Aggregate trading results across strategies
    Aggregate,
    /// Audit copy trading algorithm performance
    Audit,
    /// Fetch and cache historical trade data
    FetchHistorical {
        /// Number of days to fetch (optional)
        days: Option<u64>,
        /// Force refresh (bypass cache)
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    
    let cli = Cli::parse();

    match cli.group {
        CommandGroup::Setup { command } => handle_setup(command),
        CommandGroup::Main { command } => handle_main(command),
        CommandGroup::Wallet { command } => handle_wallet(command).await,
        CommandGroup::Position { command } => handle_position(command).await,
        CommandGroup::Research { command } => handle_research(command),
        CommandGroup::Simulation { command } => handle_simulation(command),
    }
}

fn prompt_input(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn prompt_password(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn is_valid_ethereum_address(addr: &str) -> bool {
    let clean = addr.trim().trim_start_matches("0x");
    clean.len() == 40 && clean.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_private_key(key: &str) -> bool {
    let clean = key.trim().trim_start_matches("0x");
    clean.len() == 64 && clean.chars().all(|c| c.is_ascii_hexdigit())
}

fn run_setup_wizard() -> Result<()> {
    println!("\n{}", "=".repeat(70));
    println!("POLYMARKET COPY TRADING BOT - SETUP WIZARD");
    println!("{}", "=".repeat(70));
    println!("\nThis wizard will help you create your .env configuration file.");
    println!("Press Ctrl+C at any time to cancel.\n");

    // STEP 1: Target whale address
    println!("{}", "-".repeat(70));
    println!("STEP 1: TARGET WHALE ADDRESS TO COPY");
    println!("{}", "-".repeat(70));
    println!("Find top traders on:");
    println!("  - https://polymarket.com/leaderboard");
    println!("  - https://predictfolio.com\n");
    
    let mut whale_addresses = Vec::new();
    loop {
        let prompt = if whale_addresses.is_empty() {
            "Enter trader wallet address (or press Enter to finish): "
        } else {
            "Enter another trader address (or press Enter to finish): "
        };
        
        let input = prompt_input(prompt)?;
        if input.is_empty() {
            if whale_addresses.is_empty() {
                println!("[ERROR] You must add at least one trader address!\n");
                continue;
            }
            break;
        }
        
        if !is_valid_ethereum_address(&input) {
            println!("[ERROR] Invalid Ethereum address format. Should be 0x followed by 40 hex characters.\n");
            continue;
        }
        
        let clean_addr = input.trim().trim_start_matches("0x").to_lowercase();
        whale_addresses.push(clean_addr);
        println!("[OK] Added: {}\n", input);
    }
    
    println!("[OK] Total traders to copy: {}\n", whale_addresses.len());

    // STEP 2: Wallet configuration
    println!("{}", "-".repeat(70));
    println!("STEP 2: YOUR TRADING WALLET");
    println!("{}", "-".repeat(70));
    println!("[WARNING] IMPORTANT SECURITY TIPS:");
    println!("  - Use a DEDICATED wallet for the bot");
    println!("  - Never use your main wallet");
    println!("  - Only keep trading capital in this wallet");
    println!("  - Never share your private key!\n");
    
    let wallet_address = loop {
        let input = prompt_input("Enter your Polygon wallet address: ")?;
        if is_valid_ethereum_address(&input) {
            break input.trim().trim_start_matches("0x").to_lowercase();
        }
        println!("[ERROR] Invalid wallet address format\n");
    };
    println!("[OK] Wallet: {}\n", wallet_address);
    
    let private_key = loop {
        let input = prompt_password("Enter your private key (without 0x prefix): ")?;
        if is_valid_private_key(&input) {
            break input.trim().trim_start_matches("0x").to_lowercase();
        }
        println!("[ERROR] Invalid private key format\n");
    };
    println!("[OK] Private key saved\n");

    // STEP 3: RPC endpoint
    println!("{}", "-".repeat(70));
    println!("STEP 3: POLYGON RPC ENDPOINT");
    println!("{}", "-".repeat(70));
    println!("Get free RPC endpoint from:");
    println!("  - Infura: https://infura.io (recommended)");
    println!("  - Alchemy: https://www.alchemy.com");
    println!("  - Ankr: https://www.ankr.com\n");
    
    let rpc_key = loop {
        let input = prompt_input("Enter your RPC API key: ")?;
        if !input.trim().is_empty() {
            break input.trim().to_string();
        }
        println!("[ERROR] RPC key cannot be empty\n");
    };
    println!("[OK] RPC key saved\n");

    // STEP 4: Trading strategy
    println!("{}", "-".repeat(70));
    println!("STEP 4: TRADING STRATEGY");
    println!("{}", "-".repeat(70));
    
    let use_defaults = prompt_input("Use default strategy settings? (Y/n): ")?;
    let (copy_strategy, copy_size, trade_multiplier, adaptive_min, adaptive_max, adaptive_threshold) = 
        if use_defaults.to_lowercase() == "n" || use_defaults.to_lowercase() == "no" {
            println!("\nCopy Strategy Options:");
            println!("  1. PERCENTAGE - Copy as % of trader position (recommended)");
            println!("  2. FIXED - Fixed dollar amount per trade");
            println!("  3. ADAPTIVE - Adjust based on trade size\n");
            
            let choice = prompt_input("Choose strategy (1-3, default 1): ")?;
            let strategy = match choice.trim() {
                "2" => "FIXED",
                "3" => "ADAPTIVE",
                _ => "PERCENTAGE",
            };
            
            let size_prompt = if strategy == "FIXED" {
                "Copy size in USD (default 50.0): "
            } else {
                "Copy size in % (default 10.0): "
            };
            let size_str = prompt_input(size_prompt)?;
            let size: f64 = size_str.trim().parse().unwrap_or(if strategy == "FIXED" { 50.0 } else { 10.0 });
            
            let mult_str = prompt_input("Trade multiplier (1.0 = normal, 2.0 = 2x aggressive, 0.5 = conservative, default 1.0): ")?;
            let mult: f64 = mult_str.trim().parse().unwrap_or(1.0);
            
            let (min_p, max_p, threshold) = if strategy == "ADAPTIVE" {
                let min_str = prompt_input("Adaptive min % (default 5.0): ")?;
                let max_str = prompt_input("Adaptive max % (default 15.0): ")?;
                let thresh_str = prompt_input("Adaptive threshold in USD (default 500.0): ")?;
                (
                    min_str.trim().parse().unwrap_or(5.0),
                    max_str.trim().parse().unwrap_or(15.0),
                    thresh_str.trim().parse().unwrap_or(500.0),
                )
            } else {
                (5.0, 15.0, 500.0)
            };
            
            (strategy, size, mult, min_p, max_p, threshold)
        } else {
            println!("[OK] Using default strategy: PERCENTAGE, 10%, 1.0x multiplier");
            ("PERCENTAGE", 10.0, 1.0, 5.0, 15.0, 500.0)
        };

    // STEP 5: Risk limits
    println!("\n{}", "-".repeat(70));
    println!("STEP 5: RISK LIMITS");
    println!("{}", "-".repeat(70));
    
    let use_default_limits = prompt_input("Use default risk limits? (Y/n): ")?;
    let (max_order, min_order, max_position, max_daily) = if use_default_limits.to_lowercase() == "n" || use_default_limits.to_lowercase() == "no" {
        let max_str = prompt_input("Maximum order size in USD (default 100.0): ")?;
        let min_str = prompt_input("Minimum order size in USD (default 1.0): ")?;
        let max_pos_str = prompt_input("Maximum position size per market in USD (optional, press Enter to skip): ")?;
        let max_daily_str = prompt_input("Maximum daily trading volume in USD (optional, press Enter to skip): ")?;
        (
            max_str.trim().parse().unwrap_or(100.0),
            min_str.trim().parse().unwrap_or(1.0),
            max_pos_str.trim().parse::<f64>().ok(),
            max_daily_str.trim().parse::<f64>().ok(),
        )
    } else {
        println!("[OK] Using default limits: Max $100, Min $1");
        (100.0, 1.0, None, None)
    };
    
    // STEP 6: Optional tiered multipliers
    println!("\n{}", "-".repeat(70));
    println!("STEP 6: TIERED MULTIPLIERS (OPTIONAL)");
    println!("{}", "-".repeat(70));
    println!("Tiered multipliers allow different multipliers based on trader order size.");
    println!("Format: \"min-max:multiplier,min-max:multiplier,min+:multiplier\"");
    println!("Example: \"1-10:2.0,10-100:1.0,100-500:0.5,500+:0.2\"\n");
    
    let tiered_multipliers = prompt_input("Enter tiered multipliers (optional, press Enter to skip): ")?;
    let tiered_multipliers = if tiered_multipliers.trim().is_empty() {
        None
    } else {
        Some(tiered_multipliers.trim().to_string())
    };

    // Generate .env file
    println!("\n{}", "-".repeat(70));
    println!("CREATING CONFIGURATION FILE");
    println!("{}", "-".repeat(70));
    
    let env_path = Path::new(".env");
    let env_example_path = Path::new(".env.example");
    
    // Read preserved section from .env.example (lines 47-82) if it exists
    let mut preserved_section = String::new();
    if env_example_path.exists() {
        if let Ok(content) = fs::read_to_string(env_example_path) {
            let lines: Vec<&str> = content.lines().collect();
            // Lines 47-82 (1-indexed, so 0-indexed: 46-81)
            if lines.len() > 81 {
                preserved_section = lines[46..=81].join("\n");
                if !preserved_section.is_empty() {
                    preserved_section.push('\n');
                }
            } else if lines.len() > 46 {
                // If file is shorter, preserve from line 47 to end
                preserved_section = lines[46..].join("\n");
                if !preserved_section.is_empty() {
                    preserved_section.push('\n');
                }
            }
        }
    }
    
    if env_path.exists() {
        let overwrite = prompt_input("[WARNING] .env file already exists. Overwrite? (y/N): ")?;
        if overwrite.to_lowercase() != "y" && overwrite.to_lowercase() != "yes" {
            println!("\n[INFO] Setup cancelled. Your existing .env file was not modified.");
            return Ok(());
        }
        
        // Backup existing file with timestamp
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let backup_path = format!(".env.backup.{}", timestamp);
        if let Ok(_) = fs::copy(env_path, &backup_path) {
            println!("[OK] Backed up existing .env to {}", backup_path);
        }
    }
    
    // Build .env content
    let mut env_content = String::new();
    env_content.push_str("# ================================================================\n");
    env_content.push_str("# POLYMARKET COPY TRADING BOT - CONFIGURATION\n");
    env_content.push_str("# Generated by setup wizard\n");
    env_content.push_str("# ================================================================\n\n");
    
    env_content.push_str("# ================================================================\n");
    env_content.push_str("# TRADERS TO COPY\n");
    env_content.push_str("# ================================================================\n");
    env_content.push_str(&format!("TARGET_WHALE_ADDRESS={}\n\n", whale_addresses.join(",")));
    
    env_content.push_str("# ================================================================\n");
    env_content.push_str("# YOUR WALLET\n");
    env_content.push_str("# ================================================================\n");
    env_content.push_str(&format!("PRIVATE_KEY={}\n", private_key));
    env_content.push_str(&format!("FUNDER_ADDRESS={}\n\n", wallet_address));
    
    env_content.push_str("# ================================================================\n");
    env_content.push_str("# RPC PROVIDER\n");
    env_content.push_str("# ================================================================\n");
    env_content.push_str(&format!("ALCHEMY_API_KEY={}\n\n", rpc_key));
    
    env_content.push_str("# ================================================================\n");
    env_content.push_str("# TRADING STRATEGY\n");
    env_content.push_str("# ================================================================\n");
    env_content.push_str(&format!("COPY_STRATEGY={}\n", copy_strategy));
    env_content.push_str(&format!("COPY_SIZE={}\n", copy_size));
    env_content.push_str(&format!("TRADE_MULTIPLIER={}\n", trade_multiplier));
    
    // Always include ADAPTIVE parameters (commented if not used)
    if copy_strategy == "ADAPTIVE" {
        env_content.push_str(&format!("ADAPTIVE_MIN_PERCENT={}\n", adaptive_min));
        env_content.push_str(&format!("ADAPTIVE_MAX_PERCENT={}\n", adaptive_max));
        env_content.push_str(&format!("ADAPTIVE_THRESHOLD_USD={}\n", adaptive_threshold));
    } else {
        env_content.push_str("# ADAPTIVE strategy parameters (only used when COPY_STRATEGY=ADAPTIVE)\n");
        env_content.push_str(&format!("# ADAPTIVE_MIN_PERCENT={}\n", adaptive_min));
        env_content.push_str(&format!("# ADAPTIVE_MAX_PERCENT={}\n", adaptive_max));
        env_content.push_str(&format!("# ADAPTIVE_THRESHOLD_USD={}\n", adaptive_threshold));
    }
    
    // Tiered multipliers (optional)
    if let Some(ref tiers) = tiered_multipliers {
        env_content.push_str(&format!("TIERED_MULTIPLIERS={}\n", tiers));
    } else {
        env_content.push_str("# Optional: Tiered multipliers based on trader order size\n");
        env_content.push_str("# Format: \"min-max:multiplier,min-max:multiplier,min+:multiplier\"\n");
        env_content.push_str("# Example: \"1-10:2.0,10-100:1.0,100-500:0.5,500+:0.2\"\n");
        env_content.push_str("# TIERED_MULTIPLIERS=\n");
    }
    env_content.push_str("\n");
    
    env_content.push_str("# ================================================================\n");
    env_content.push_str("# RISK LIMITS\n");
    env_content.push_str("# ================================================================\n");
    env_content.push_str(&format!("MAX_ORDER_SIZE_USD={}\n", max_order));
    env_content.push_str(&format!("MIN_ORDER_SIZE_USD={}\n", min_order));
    
    // Optional risk limits
    if let Some(max_pos) = max_position {
        env_content.push_str(&format!("MAX_POSITION_SIZE_USD={}\n", max_pos));
    } else {
        env_content.push_str("# Optional: Maximum position size per market in USD\n");
        env_content.push_str("# MAX_POSITION_SIZE_USD=\n");
    }
    
    if let Some(max_daily) = max_daily {
        env_content.push_str(&format!("MAX_DAILY_VOLUME_USD={}\n", max_daily));
    } else {
        env_content.push_str("# Optional: Maximum daily trading volume in USD\n");
        env_content.push_str("# MAX_DAILY_VOLUME_USD=\n");
    }
    env_content.push_str("\n");
    
    env_content.push_str("# ================================================================\n");
    env_content.push_str("# TRADING FLAGS\n");
    env_content.push_str("# ================================================================\n");
    env_content.push_str("ENABLE_TRADING=true\n");
    env_content.push_str("MOCK_TRADING=false\n");
    
    // Append preserved section from .env.example if available
    if !preserved_section.is_empty() {
        env_content.push_str("\n");
        env_content.push_str("# ================================================================\n");
        env_content.push_str("# PRESERVED FROM .env.example (Advanced Settings)\n");
        env_content.push_str("# ================================================================\n");
        env_content.push_str(&preserved_section);
    }
    
    // Write .env file
    fs::write(env_path, env_content)?;
    
    // Success message
    println!("\n{}", "=".repeat(70));
    println!("SETUP COMPLETE");
    println!("{}", "=".repeat(70));
    println!("\nConfiguration saved to: .env\n");
    println!("PRE-FLIGHT CHECKLIST:\n");
    println!("  [ ] Fund your wallet with USDC on Polygon");
    println!("  [ ] Get POL (MATIC) for gas fees (~$5-10)");
    println!("  [ ] Verify traders are actively trading");
    println!("\nNEXT STEPS:\n");
    println!("  1. Review your .env file");
    println!("  2. Install dependencies: cargo build --release");
    println!("  3. Run system status check: cargo run --release setup system-status");
    println!("  4. Start trading: cargo run --release main run\n");
    println!("[WARNING] REMEMBER:");
    println!("  - Start with small amounts to test");
    println!("  - Monitor the bot regularly");
    println!("  - Only trade what you can afford to lose\n");
    println!("[OK] Setup complete. Ready to start trading.\n");
    
    Ok(())
}

fn run_system_status() -> Result<()> {
    println!("📊 System Status Check");
    println!("{}", "=".repeat(70));
    println!();
    
    // Load and display configuration
    match Config::from_env() {
        Ok(config) => {
            println!("✅ Configuration loaded successfully\n");
            
            // Display strategy information
            println!("{}", "-".repeat(70));
            println!("TRADING STRATEGY");
            println!("{}", "-".repeat(70));
            match config.copy_strategy {
                CopyStrategy::Percentage => {
                    println!("  Strategy: PERCENTAGE");
                    println!("  Copy Size: {:.1}% of trader order", config.copy_size);
                }
                CopyStrategy::Fixed => {
                    println!("  Strategy: FIXED");
                    println!("  Copy Size: ${:.2} per trade", config.copy_size);
                }
                CopyStrategy::Adaptive => {
                    println!("  Strategy: ADAPTIVE");
                    println!("  Base %: {:.1}%", config.copy_size);
                    println!("  Min %: {:.1}% (for large orders)", config.adaptive_min_percent);
                    println!("  Max %: {:.1}% (for small orders)", config.adaptive_max_percent);
                    println!("  Threshold: ${:.2}", config.adaptive_threshold_usd);
                }
            }
            println!("  Trade Multiplier: {:.2}x", config.trade_multiplier);
            if let Some(ref tiers) = config.tiered_multipliers {
                println!("  Tiered Multipliers: {}", tiers);
            }
            println!();
            
            // Display risk limits
            println!("{}", "-".repeat(70));
            println!("RISK LIMITS");
            println!("{}", "-".repeat(70));
            println!("  Max Order Size: ${:.2}", config.max_order_size_usd);
            println!("  Min Order Size: ${:.2}", config.min_order_size_usd);
            if let Some(max_pos) = config.max_position_size_usd {
                println!("  Max Position Size: ${:.2}", max_pos);
            }
            if let Some(max_daily) = config.max_daily_volume_usd {
                println!("  Max Daily Volume: ${:.2}", max_daily);
            }
            println!();
            
            // Display trading flags
            println!("{}", "-".repeat(70));
            println!("TRADING FLAGS");
            println!("{}", "-".repeat(70));
            println!("  Trading Enabled: {}", config.enable_trading);
            println!("  Mock Trading: {}", config.mock_trading);
            println!();
            
            // Display wallet info (masked)
            println!("{}", "-".repeat(70));
            println!("WALLET CONFIGURATION");
            println!("{}", "-".repeat(70));
            let funder_display = if config.funder_address.len() > 10 {
                format!("{}...{}", &config.funder_address[..6], &config.funder_address[config.funder_address.len()-4..])
            } else {
                config.funder_address.clone()
            };
            println!("  Funder Address: {}", funder_display);
            let key_display = if config.private_key.len() > 10 {
                format!("{}...{}", &config.private_key[..6], &config.private_key[config.private_key.len()-4..])
            } else {
                "***".to_string()
            };
            println!("  Private Key: {} (masked)", key_display);
            println!();
        }
        Err(e) => {
            println!("❌ Configuration Error:");
            println!("   {}\n", e);
            return Err(anyhow!("Configuration validation failed"));
        }
    }
    
    // Run validation checks
    println!("{}", "-".repeat(70));
    println!("RUNNING VALIDATION CHECKS");
    println!("{}", "-".repeat(70));
    println!();
    
    println!("Checking configuration format...");
    let status1 = std::process::Command::new("cargo")
        .args(&["run", "--release", "--bin", "validate_setup"])
        .status()?;
    if !status1.success() {
        return Err(anyhow::anyhow!("Configuration validation failed"));
    }
    
    println!("\nChecking balance...");
    let status2 = std::process::Command::new("cargo")
        .args(&["run", "--release", "--bin", "check_balance"])
        .status()?;
    if !status2.success() {
        return Err(anyhow::anyhow!("Balance check failed"));
    }
    
    println!("\nTesting connections...");
    let status3 = std::process::Command::new("cargo")
        .args(&["run", "--release", "--bin", "test_connection"])
        .status()?;
    if !status3.success() {
        return Err(anyhow::anyhow!("Connection test failed"));
    }
    
    println!("\n{}", "=".repeat(70));
    println!("✅ All checks passed! System is ready.");
    println!("{}", "=".repeat(70));
    println!();
    
    Ok(())
}

fn handle_setup(cmd: SetupCommand) -> Result<()> {
    match cmd {
        SetupCommand::Setup => {
            run_setup_wizard()
        }
        SetupCommand::SystemStatus => {
            run_system_status()
        }
        SetupCommand::Help => {
            print_help();
            Ok(())
        }
    }
}

fn handle_main(cmd: MainCommand) -> Result<()> {
    match cmd {
        MainCommand::Run => {
            // Delegate to pm_bot binary (main.rs)
            println!("🚀 Starting Polymarket Copy Trading Bot\n");
            // Run the pm_bot binary which contains the main bot logic
            let status = std::process::Command::new("cargo")
                .args(&["run", "--release", "--bin", "pm_bot"])
                .status()?;
            if !status.success() {
                return Err(anyhow::anyhow!("Bot execution failed with exit code: {:?}", status.code()));
            }
            Ok(())
        }
    }
}

async fn handle_wallet(cmd: WalletCommand) -> Result<()> {
    match cmd {
        WalletCommand::CheckProxyWallet => {
            println!("🏦 Checking Proxy Wallet (Gnosis Safe)");
            println!("=====================================\n");
            // Delegate to check_balance for now
            std::process::Command::new("cargo")
                .args(&["run", "--release", "--bin", "check_balance"])
                .status()?;
            Ok(())
        }
        WalletCommand::CheckBothWallets { address1, address2 } => {
            println!("🔍 Comparing Wallets");
            println!("===================\n");
            println!("⚠️  TODO: Implement wallet comparison");
            println!("   Compare: {} vs {}", address1, address2);
            println!("   Should show: balance, positions, activity comparison\n");
            Ok(())
        }
        WalletCommand::CheckMyStats => {
            check_my_stats().await
        }
        WalletCommand::CheckRecentActivity => {
            check_recent_activity().await
        },
        WalletCommand::CheckPositionsDetailed => {
            check_positions_detailed().await
        }
        WalletCommand::CheckPnlDiscrepancy => {
            check_pnl_discrepancy().await
        }
        WalletCommand::VerifyAllowance => {
            println!("✅ Verifying Token Allowance");
            println!("===========================\n");
            // Delegate to approve_tokens with dry-run
            std::process::Command::new("cargo")
                .args(&["run", "--release", "--bin", "approve_tokens", "--", "--dry-run"])
                .status()?;
            Ok(())
        }
        WalletCommand::CheckAllowance => {
            println!("🔍 Checking Token Allowance");
            println!("==========================\n");
            // Delegate to approve_tokens
            std::process::Command::new("cargo")
                .args(&["run", "--release", "--bin", "approve_tokens"])
                .status()?;
            Ok(())
        }
        WalletCommand::SetTokenAllowance => {
            println!("🔧 Setting Token Allowance");
            println!("==========================\n");
            println!("⚠️  TODO: Implement ERC1155 token allowance setting");
            println!("   This should set allowance for Conditional Tokens\n");
            // For now, delegate to approve_tokens
            std::process::Command::new("cargo")
                .args(&["run", "--release", "--bin", "approve_tokens"])
                .status()?;
            Ok(())
        }
        WalletCommand::FindMyEoa => {
            find_my_eoa().await
        }
        WalletCommand::FindGnosisSafeProxy => {
            find_gnosis_safe_proxy().await
        }
    }
}

async fn handle_position(cmd: PositionCommand) -> Result<()> {
    match cmd {
        PositionCommand::ManualSell { market_id, outcome, amount } => {
            println!("💰 Manual Sell Position");
            println!("======================\n");
            println!("⚠️  TODO: Implement manual sell position");
            println!("   Market ID: {}", market_id);
            println!("   Outcome: {}", outcome);
            println!("   Amount: {}", amount);
            println!("\n   Required logic:");
            println!("   1. Validate market_id and outcome");
            println!("   2. Check position exists");
            println!("   3. Build sell order using CLOB client");
            println!("   4. Submit order via authenticated client");
            println!("   5. Monitor fill status\n");
            Ok(())
        }
        PositionCommand::SellLarge => {
            sell_large_positions().await
        }
        PositionCommand::CloseStale => {
            close_stale_positions().await
        }
        PositionCommand::CloseResolved => {
            close_resolved_positions().await
        }
        PositionCommand::RedeemResolved => {
            redeem_resolved_positions().await
        }
    }
}

fn handle_research(cmd: ResearchCommand) -> Result<()> {
    match cmd {
        ResearchCommand::FindBestTraders => {
            println!("🏆 Find Best Traders");
            println!("===================\n");
            println!("⚠️  TODO: Implement find best traders");
            println!("   Required logic:");
            println!("   1. Query Polymarket leaderboards/API");
            println!("   2. Calculate performance metrics (ROI, win rate, P&L)");
            println!("   3. Rank traders by performance");
            println!("   4. Display ranking table\n");
            Ok(())
        }
        ResearchCommand::FindLowRiskTraders => {
            println!("🛡️  Find Low-Risk Traders");
            println!("========================\n");
            println!("⚠️  TODO: Implement find low-risk traders");
            println!("   Required logic:");
            println!("   1. Query trader performance data");
            println!("   2. Calculate risk metrics (Sharpe ratio, drawdown, etc.)");
            println!("   3. Filter by risk criteria");
            println!("   4. Display conservative performers\n");
            Ok(())
        }
        ResearchCommand::ScanBestTraders => {
            println!("🔍 Scan Best Traders");
            println!("===================\n");
            println!("⚠️  TODO: Implement scan best traders");
            println!("   Required logic:");
            println!("   1. Scan active markets");
            println!("   2. Identify top traders");
            println!("   3. Analyze performance");
            println!("   4. Generate report\n");
            Ok(())
        }
        ResearchCommand::ScanFromMarkets => {
            println!("📈 Scan Traders from Markets");
            println!("============================\n");
            println!("⚠️  TODO: Implement scan traders from markets");
            println!("   Required logic:");
            println!("   1. Scan active markets");
            println!("   2. Extract trader addresses");
            println!("   3. Analyze activity");
            println!("   4. Generate trader list\n");
            Ok(())
        }
    }
}

fn handle_simulation(cmd: SimulationCommand) -> Result<()> {
    match cmd {
        SimulationCommand::SimulateProfitability { trader_address } => {
            println!("📊 Simulate Profitability");
            println!("=========================\n");
            println!("⚠️  TODO: Implement profitability simulation");
            println!("   Trader: {:?}", trader_address);
            println!("   Required logic:");
            println!("   1. Fetch trader historical trades");
            println!("   2. Simulate copying each trade with bot's strategy");
            println!("   3. Calculate ROI, P&L, win rate");
            println!("   4. Generate report\n");
            Ok(())
        }
        SimulationCommand::SimulateProfitabilityOld { trader_address } => {
            println!("📊 Simulate Profitability (Old Logic)");
            println!("====================================\n");
            println!("⚠️  TODO: Implement old profitability simulation");
            println!("   Trader: {:?}", trader_address);
            println!("   Uses legacy simulation algorithm\n");
            Ok(())
        }
        SimulationCommand::Run { preset } => {
            println!("🚀 Run Simulations");
            println!("=================\n");
            println!("⚠️  TODO: Implement batch simulations");
            println!("   Preset: {:?}", preset);
            println!("   Required logic:");
            println!("   1. Support presets: quick, standard, full, custom");
            println!("   2. Run simulations for multiple traders");
            println!("   3. Compare strategies");
            println!("   4. Save results\n");
            Ok(())
        }
        SimulationCommand::Compare { mode } => {
            println!("📊 Compare Results");
            println!("=================\n");
            println!("⚠️  TODO: Implement result comparison");
            println!("   Mode: {:?}", mode);
            println!("   Required logic:");
            println!("   1. Load simulation results");
            println!("   2. Compare based on mode (best, worst, stats, detail)");
            println!("   3. Display comparison table\n");
            Ok(())
        }
        SimulationCommand::Aggregate => {
            println!("📈 Aggregate Results");
            println!("===================\n");
            println!("⚠️  TODO: Implement result aggregation");
            println!("   Required logic:");
            println!("   1. Scan result directories");
            println!("   2. Aggregate statistics across strategies");
            println!("   3. Generate summary report");
            println!("   4. Save to strategy_factory_results/\n");
            Ok(())
        }
        SimulationCommand::Audit => {
            println!("🔍 Audit Copy Trading");
            println!("====================\n");
            println!("⚠️  TODO: Implement copy trading audit");
            println!("   Required logic:");
            println!("   1. Simulate trading with bot's algorithm");
            println!("   2. Compare with actual bot performance");
            println!("   3. Identify discrepancies");
            println!("   4. Generate audit report\n");
            Ok(())
        }
        SimulationCommand::FetchHistorical { days, force } => {
            println!("📥 Fetch Historical Trades");
            println!("==========================\n");
            println!("⚠️  TODO: Implement historical trade fetching");
            println!("   Days: {:?}", days);
            println!("   Force: {}", force);
            println!("   Required logic:");
            println!("   1. Fetch trader history from API");
            println!("   2. Cache to trader_data_cache/");
            println!("   3. Support parallel processing");
            println!("   4. Handle rate limiting\n");
            Ok(())
        }
    }
}

fn print_help() {
    println!("Polymarket Copy Trading Bot CLI");
    println!("================================\n");
    println!("Available Commands:\n");
    
    println!("📋 Setup & Configuration:");
    println!("  cargo run --release setup setup               - Interactive setup wizard");
    println!("  cargo run --release setup system-status       - Validate config, check balances, connectivity");
    println!("  cargo run --release setup help                - Print this help message\n");
    
    println!("🚀 Main Bot:");
    println!("  cargo run --release main run                  - Start the copy trading bot\n");
    
    println!("💼 Wallet Management:");
    println!("  cargo run --release wallet check-proxy-wallet         - Check Gnosis Safe balance/positions");
    println!("  cargo run --release wallet check-both-wallets <a1> <a2> - Compare two wallets");
    println!("  cargo run --release wallet check-my-stats             - View wallet statistics");
    println!("  cargo run --release wallet check-recent-activity      - View recent trades");
    println!("  cargo run --release wallet check-positions-detailed   - View detailed positions");
    println!("  cargo run --release wallet check-pnl-discrepancy      - Analyze P&L discrepancies");
    println!("  cargo run --release wallet verify-allowance           - Verify token allowance");
    println!("  cargo run --release wallet check-allowance            - Check and set allowance");
    println!("  cargo run --release wallet set-token-allowance        - Set ERC1155 allowance");
    println!("  cargo run --release wallet find-my-eoa                - Find EOA wallet");
    println!("  cargo run --release wallet find-gnosis-safe-proxy     - Find Gnosis Safe proxy\n");
    
    println!("💰 Position Management:");
    println!("  cargo run --release position manual-sell <market> <outcome> <amount>");
    println!("  cargo run --release position sell-large               - Sell large positions");
    println!("  cargo run --release position close-stale              - Close old positions");
    println!("  cargo run --release position close-resolved           - Close resolved positions");
    println!("  cargo run --release position redeem-resolved          - Redeem resolved positions\n");
    
    println!("🔍 Trader Research:");
    println!("  cargo run --release research find-best-traders        - Find top performers");
    println!("  cargo run --release research find-low-risk-traders    - Find low-risk traders");
    println!("  cargo run --release research scan-best-traders        - Scan top traders");
    println!("  cargo run --release research scan-from-markets        - Scan from markets\n");
    
    println!("📊 Simulation & Backtesting:");
    println!("  cargo run --release simulation simulate-profitability [trader]");
    println!("  cargo run --release simulation simulate-profitability-old [trader]");
    println!("  cargo run --release simulation run [preset]           - Run batch simulations");
    println!("  cargo run --release simulation compare [mode]         - Compare results");
    println!("  cargo run --release simulation aggregate              - Aggregate results");
    println!("  cargo run --release simulation audit                  - Audit algorithm");
    println!("  cargo run --release simulation fetch-historical [--force] [--days N]\n");
    
    println!("For detailed help on a command, use:");
    println!("  cargo run --release -- <group> --help              # Help for a command group");
    println!("  cargo run --release -- <group> <command> --help   # Help for a specific command\n");
    println!("Note: Use '--' to separate Cargo arguments from binary arguments.\n");
}

const USDC_ADDRESS: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
const CSV_FILE: &str = "matches_optimized.csv";
const DEFAULT_RPC_URL: &str = "https://polygon-rpc.com";

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
    }
}

#[derive(Deserialize, Clone)]
struct CsvRow {
    #[serde(rename = "timestamp")]
    #[allow(dead_code)]
    timestamp: Option<String>,
    #[serde(rename = "direction")]
    direction: Option<String>,
    #[serde(rename = "shares")]
    shares: Option<String>,
    #[serde(rename = "price_per_share")]
    price_per_share: Option<String>,
    #[serde(rename = "order_status")]
    order_status: Option<String>,
    #[serde(rename = "usd_value")]
    usd_value: Option<String>,
    #[serde(rename = "clob_asset_id")]
    clob_asset_id: Option<String>,
}

async fn check_my_stats() -> Result<()> {
    dotenvy::dotenv().ok();
    
    println!("📊 Wallet Statistics");
    println!("===================\n");

    // Load configuration
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set"))?;
    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    
    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| signer.address());

    println!("📝 Wallet Address: {}\n", funder_address);

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

    // Get balances
    let provider = ProviderBuilder::new()
        .wallet(signer.clone())
        .connect_http(rpc_url.parse()?);

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

    let usdc_addr = Address::from_str(USDC_ADDRESS)?;
    let usdc = IERC20::new(usdc_addr, provider.clone());
    let usdc_balance = usdc.balanceOf(funder_address).call().await?;
    let usdc_balance_formatted = format_units(usdc_balance, 6);

    println!("💰 Balance Summary:");
    println!("   USDC: {} USDC", usdc_balance_formatted);
    println!("   MATIC: {} MATIC\n", matic_balance_eth);

    // Analyze CSV file for trading statistics
    if Path::new(CSV_FILE).exists() {
        let csv_content = fs::read_to_string(CSV_FILE)?;
        let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
        
        let mut total_trades = 0;
        let mut buy_trades = 0;
        let mut sell_trades = 0;
        let mut successful_trades = 0;
        let mut total_volume = 0.0;
        let mut recent_trades = Vec::new();

        for result in reader.deserialize::<CsvRow>() {
            if let Ok(row) = result {
                total_trades += 1;
                
                // Check direction (BUY_FILL, SELL_FILL, etc.)
                if let Some(ref direction) = row.direction {
                    if direction.contains("BUY") {
                        buy_trades += 1;
                    } else if direction.contains("SELL") {
                        sell_trades += 1;
                    }
                }
                
                // Check order status (successful trades don't have "SKIPPED" in status)
                if let Some(ref status) = row.order_status {
                    if !status.contains("SKIPPED") {
                        successful_trades += 1;
                    }
                }
                
                // Calculate volume from usd_value if available, otherwise from shares * price
                if let Some(ref usd_val) = row.usd_value {
                    if let Ok(vol) = usd_val.parse::<f64>() {
                        total_volume += vol;
                    }
                } else if let (Some(ref shares), Some(ref price)) = (row.shares.as_ref(), row.price_per_share.as_ref()) {
                    if let (Ok(shares_val), Ok(price_val)) = (shares.parse::<f64>(), price.parse::<f64>()) {
                        total_volume += shares_val * price_val;
                    }
                }
                
                // Keep last 10 trades for recent activity
                if recent_trades.len() < 10 {
                    recent_trades.push(row);
                } else {
                    recent_trades.remove(0);
                    recent_trades.push(row);
                }
            }
        }

        println!("📈 Trading Statistics:");
        println!("   Total Trades: {}", total_trades);
        println!("   Buy Trades: {}", buy_trades);
        println!("   Sell Trades: {}", sell_trades);
        println!("   Successful: {} ({:.1}%)", successful_trades, 
                 if total_trades > 0 { (successful_trades as f64 / total_trades as f64) * 100.0 } else { 0.0 });
        println!("   Total Volume: ${:.2} USDC\n", total_volume);

        if !recent_trades.is_empty() {
            println!("🕐 Recent Activity (last {} trades):", recent_trades.len().min(5));
            for (i, trade) in recent_trades.iter().rev().take(5).enumerate() {
                let direction = trade.direction.as_deref().unwrap_or("?");
                let shares = trade.shares.as_deref().unwrap_or("?");
                let price = trade.price_per_share.as_deref().unwrap_or("?");
                let status = trade.order_status.as_deref().unwrap_or("?");
                let usd_val = trade.usd_value.as_deref().unwrap_or("?");
                
                // Format status - truncate if too long, but show key info
                let status_display = if status.len() > 50 {
                    // Try to show the important part (error type or status code)
                    if status.contains("200 OK") {
                        "200 OK".to_string()
                    } else if status.contains("EXEC_FAIL") {
                        let fail_part = status.split("EXEC_FAIL:").nth(1).unwrap_or(status);
                        if fail_part.len() > 45 {
                            format!("EXEC_FAIL:{}...", &fail_part[..40])
                        } else {
                            format!("EXEC_FAIL:{}", fail_part)
                        }
                    } else if status.contains("SKIPPED") {
                        let skip_part = status.split("SKIPPED").nth(0).unwrap_or(status);
                        if skip_part.len() > 45 {
                            format!("{}...", &skip_part[..42])
                        } else {
                            status.to_string()
                        }
                    } else {
                        format!("{}...", &status[..47])
                    }
                } else {
                    status.to_string()
                };
                
                println!("   {}. {} {} shares @ ${} (${}) - {}", 
                         i + 1, direction, shares, price, usd_val, status_display);
            }
            println!();
        }
    } else {
        println!("📈 Trading Statistics:");
        println!("   No trading history found ({} not found)\n", CSV_FILE);
    }

    println!("💡 Tip: Use 'cargo run --release wallet check-recent-activity' for detailed trade history");
    Ok(())
}

async fn check_recent_activity() -> Result<()> {
    println!("📥 Recent Trading Activity");
    println!("=========================\n");

    if !Path::new(CSV_FILE).exists() {
        println!("❌ No trading history found ({} not found)", CSV_FILE);
        println!("   The bot needs to run and make trades first.\n");
        return Ok(());
    }

    let csv_content = fs::read_to_string(CSV_FILE)?;
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
    
    let mut all_trades: Vec<CsvRow> = Vec::new();
    for result in reader.deserialize::<CsvRow>() {
        if let Ok(row) = result {
            all_trades.push(row);
        }
    }

    if all_trades.is_empty() {
        println!("📭 No trades found in history\n");
        return Ok(());
    }

    // Show last 20 trades
    let recent_count = all_trades.len().min(20);
    println!("📊 Showing last {} trades:\n", recent_count);
    println!("{:-<120}", "");
    println!("{:<4} {:<12} {:<12} {:<10} {:<10} {:<12} {:<50}", 
             "#", "Time", "Direction", "Shares", "Price", "Value", "Status");
    println!("{:-<120}", "");

    for (i, trade) in all_trades.iter().rev().take(recent_count).enumerate() {
        let num = i + 1;
        let timestamp = trade.timestamp.as_deref().unwrap_or("?");
        // Format timestamp to show just time if it's long
        let time_display = if timestamp.len() > 19 {
            // Format: "2026-01-16 23:06:31.824" -> "23:06:31"
            if let Some(time_part) = timestamp.split(' ').nth(1) {
                if let Some(just_time) = time_part.split('.').next() {
                    just_time.to_string()
                } else {
                    time_part.to_string()
                }
            } else {
                timestamp.to_string()
            }
        } else if timestamp.len() > 10 {
            timestamp.split(' ').nth(1).unwrap_or(timestamp).to_string()
        } else {
            timestamp.to_string()
        };
        
        let direction = trade.direction.as_deref().unwrap_or("?");
        let shares = trade.shares.as_deref().unwrap_or("?");
        let price = trade.price_per_share.as_deref().unwrap_or("?");
        let usd_val = trade.usd_value.as_deref().unwrap_or("?");
        let status = trade.order_status.as_deref().unwrap_or("?");
        
        // Format status - truncate if too long
        let status_display = if status.len() > 48 {
            if status.contains("200 OK") {
                "200 OK".to_string()
            } else if status.contains("EXEC_FAIL") {
                let fail_part = status.split("EXEC_FAIL:").nth(1).unwrap_or(status);
                if fail_part.len() > 43 {
                    format!("EXEC_FAIL:{}...", &fail_part[..38])
                } else {
                    format!("EXEC_FAIL:{}", fail_part)
                }
            } else if status.contains("SKIPPED") {
                let skip_reason = status.split("SKIPPED").nth(1).unwrap_or("");
                if skip_reason.len() > 45 {
                    format!("SKIPPED{}...", &skip_reason[..40])
                } else {
                    status.to_string()
                }
            } else {
                format!("{}...", &status[..45])
            }
        } else {
            status.to_string()
        };

        println!("{:<4} {:<12} {:<12} {:<10} {:<10} {:<12} {:<50}", 
                 num, time_display, direction, shares, price, usd_val, status_display);
    }
    
    println!("{:-<120}", "");
    println!("\n💡 Tip: View full CSV file: {}", CSV_FILE);
    Ok(())
}

#[derive(Default, Clone)]
struct Position {
    token_id: String,
    total_shares: f64,
    total_cost: f64,
    last_price: f64,
    buy_count: usize,
    sell_count: usize,
}

async fn check_positions_detailed() -> Result<()> {
    dotenvy::dotenv().ok();
    
    println!("📋 Detailed Positions");
    println!("====================\n");

    // Load and display wallet address
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set"))?;
    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    
    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| signer.address());

    println!("📝 Wallet Address: {}", funder_address);
    if funder_address != signer.address() {
        println!("   ℹ️  Using Gnosis Safe as funder\n");
    } else {
        println!();
    }

    if !Path::new(CSV_FILE).exists() {
        println!("❌ No trading history found ({} not found)", CSV_FILE);
        println!("   The bot needs to run and make trades first.\n");
        return Ok(());
    }

    let csv_content = fs::read_to_string(CSV_FILE)?;
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
    
    let mut positions: std::collections::HashMap<String, Position> = std::collections::HashMap::new();
    
    for result in reader.deserialize::<CsvRow>() {
        if let Ok(row) = result {
            // Only process successful trades (not SKIPPED)
            if let Some(ref status) = row.order_status {
                if status.contains("SKIPPED") {
                    continue;
                }
            }
            
            let token_id = row.clob_asset_id.as_deref().unwrap_or("unknown").to_string();
            let direction = row.direction.as_deref().unwrap_or("?");
            let shares_str = row.shares.as_deref().unwrap_or("0");
            let price_str = row.price_per_share.as_deref().unwrap_or("0");
            let usd_val_str = row.usd_value.as_deref().unwrap_or("0");
            
            let shares = shares_str.parse::<f64>().unwrap_or(0.0);
            let price = price_str.parse::<f64>().unwrap_or(0.0);
            let usd_val = usd_val_str.parse::<f64>().unwrap_or(0.0);
            
            let pos = positions.entry(token_id.clone()).or_insert_with(|| Position {
                token_id: token_id.clone(),
                ..Default::default()
            });
            
            if direction.contains("BUY") {
                pos.total_shares += shares;
                pos.total_cost += usd_val;
                pos.buy_count += 1;
                pos.last_price = price; // Update last price
            } else if direction.contains("SELL") {
                pos.total_shares -= shares;
                pos.total_cost -= usd_val; // Reduce cost basis
                pos.sell_count += 1;
                pos.last_price = price;
            }
        }
    }

    // Filter to only show positions with shares > 0
    let mut open_positions: Vec<&Position> = positions.values()
        .filter(|p| p.total_shares > 0.001) // Filter out near-zero positions
        .collect();
    
    if open_positions.is_empty() {
        println!("📭 No open positions found\n");
        println!("   All positions have been closed or no successful trades yet.\n");
        return Ok(());
    }

    // Sort by total value (shares * last_price)
    open_positions.sort_by(|a, b| {
        let val_a = a.total_shares * a.last_price;
        let val_b = b.total_shares * b.last_price;
        val_b.partial_cmp(&val_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("📊 Found {} open position(s):\n", open_positions.len());
    println!("{:-<130}", "");
    println!("{:<20} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12}", 
             "Token ID", "Shares", "Avg Price", "Cost Basis", "Last Price", "Current Val", "P&L", "Trades");
    println!("{:-<130}", "");

    let mut total_cost = 0.0;
    let mut total_value = 0.0;

    for pos in &open_positions {
        let avg_price = if pos.total_shares > 0.0 {
            pos.total_cost / pos.total_shares
        } else {
            0.0
        };
        let current_value = pos.total_shares * pos.last_price;
        let pnl = current_value - pos.total_cost;
        let pnl_pct = if pos.total_cost > 0.0 {
            (pnl / pos.total_cost) * 100.0
        } else {
            0.0
        };
        
        total_cost += pos.total_cost;
        total_value += current_value;
        
        // Truncate token ID for display
        let token_display = if pos.token_id.len() > 18 {
            format!("{}...", &pos.token_id[..15])
        } else {
            pos.token_id.clone()
        };
        
        let pnl_sign = if pnl >= 0.0 { "+" } else { "" };
        println!("{:<20} {:<12.6} ${:<11.4} ${:<11.2} ${:<11.4} ${:<11.2} {}{:<11.2} ({:+.1}%) {:<3}B/{:<3}S", 
                 token_display,
                 pos.total_shares,
                 avg_price,
                 pos.total_cost,
                 pos.last_price,
                 current_value,
                 pnl_sign,
                 pnl,
                 pnl_pct,
                 pos.buy_count,
                 pos.sell_count);
    }
    
    println!("{:-<130}", "");
    let total_pnl = total_value - total_cost;
    let total_pnl_pct = if total_cost > 0.0 {
        (total_pnl / total_cost) * 100.0
    } else {
        0.0
    };
    let total_pnl_sign = if total_pnl >= 0.0 { "+" } else { "" };
    println!("{:<20} {:<12} {:<12} ${:<11.2} {:<12} ${:<11.2} {}{:<11.2} ({:+.1}%)", 
             "TOTAL", "", "", total_cost, "", total_value, total_pnl_sign, total_pnl, total_pnl_pct);
    println!("{:-<130}", "");
    
    println!("\n💡 Note: Current value uses last trade price. For real-time prices, check Polymarket directly.");
    println!("💡 Tip: Use 'cargo run --release wallet check-my-stats' for overall statistics\n");
    
    Ok(())
}

async fn check_pnl_discrepancy() -> Result<()> {
    dotenvy::dotenv().ok();
    
    println!("🔍 P&L Discrepancy Analysis");
    println!("==========================\n");

    // Load wallet address
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set"))?;
    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    
    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| signer.address());

    println!("📝 Wallet Address: {}", funder_address);
    if funder_address != signer.address() {
        println!("   ℹ️  Using Gnosis Safe as funder\n");
    } else {
        println!();
    }

    if !Path::new(CSV_FILE).exists() {
        println!("❌ No trading history found ({} not found)", CSV_FILE);
        println!("   The bot needs to run and make trades first.\n");
        return Ok(());
    }

    let csv_content = fs::read_to_string(CSV_FILE)?;
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
    
    let mut positions: std::collections::HashMap<String, Position> = std::collections::HashMap::new();
    let mut all_trades: Vec<CsvRow> = Vec::new();
    let mut total_buy_cost = 0.0;
    let mut total_sell_proceeds = 0.0;
    let mut skipped_trades = 0;
    let mut failed_trades = 0;
    
    for result in reader.deserialize::<CsvRow>() {
        if let Ok(row) = result {
            all_trades.push(row.clone());
            
            let status = row.order_status.as_deref().unwrap_or("?");
            if status.contains("SKIPPED") {
                skipped_trades += 1;
                continue;
            }
            if status.contains("EXEC_FAIL") || status.contains("error") {
                failed_trades += 1;
                continue;
            }
            
            let token_id = row.clob_asset_id.as_deref().unwrap_or("unknown").to_string();
            let direction = row.direction.as_deref().unwrap_or("?");
            let shares_str = row.shares.as_deref().unwrap_or("0");
            let usd_val_str = row.usd_value.as_deref().unwrap_or("0");
            
            let shares = shares_str.parse::<f64>().unwrap_or(0.0);
            let usd_val = usd_val_str.parse::<f64>().unwrap_or(0.0);
            
            let pos = positions.entry(token_id.clone()).or_insert_with(|| Position {
                token_id: token_id.clone(),
                ..Default::default()
            });
            
            if direction.contains("BUY") {
                pos.total_shares += shares;
                pos.total_cost += usd_val;
                pos.buy_count += 1;
                total_buy_cost += usd_val;
            } else if direction.contains("SELL") {
                pos.total_shares -= shares;
                pos.total_cost -= usd_val;
                pos.sell_count += 1;
                total_sell_proceeds += usd_val;
            }
        }
    }

    if all_trades.is_empty() {
        println!("📭 No trades found in history\n");
        return Ok(());
    }

    // Calculate current positions value
    let mut total_current_value = 0.0;
    let mut total_cost_basis = 0.0;
    let mut open_positions_count = 0;
    let mut positions_with_zero_price = 0;
    
    for pos in positions.values() {
        if pos.total_shares > 0.001 {
            // Use last_price if available, otherwise use average price as fallback
            let price = if pos.last_price > 0.0 {
                pos.last_price
            } else {
                // Fallback to average price if last_price is 0
                if pos.total_shares > 0.0 {
                    pos.total_cost / pos.total_shares
                } else {
                    0.0
                }
            };
            
            if price == 0.0 {
                positions_with_zero_price += 1;
            }
            
            total_current_value += pos.total_shares * price;
            total_cost_basis += pos.total_cost;
            open_positions_count += 1;
        }
    }

    // Calculate realized P&L from closed positions
    let realized_pnl = total_sell_proceeds - (total_buy_cost - total_cost_basis);
    
    // Calculate unrealized P&L from open positions
    let unrealized_pnl = total_current_value - total_cost_basis;
    
    // Total P&L
    let total_pnl = realized_pnl + unrealized_pnl;
    
    // Expected P&L (if all positions were closed at last price)
    let expected_pnl_if_closed = total_sell_proceeds + total_current_value - total_buy_cost;
    
    // Net cash flow (money in vs money out)
    let net_cash_flow = total_sell_proceeds - total_buy_cost;

    println!("📊 Trade Summary:");
    println!("   Total Trades: {}", all_trades.len());
    println!("   Successful Trades: {}", all_trades.len() - skipped_trades - failed_trades);
    println!("   Skipped Trades: {} (not executed)", skipped_trades);
    println!("   Failed Trades: {} (execution errors)", failed_trades);
    println!("   Open Positions: {}\n", open_positions_count);

    println!("💰 P&L Breakdown:");
    println!("   Total Buy Cost: ${:.2}", total_buy_cost);
    println!("   Total Sell Proceeds: ${:.2}", total_sell_proceeds);
    println!("   Net Cash Flow: ${:.2} (money in - money out)", net_cash_flow);
    println!("   Cost Basis (Open Positions): ${:.2}", total_cost_basis);
    println!("   Current Value (Open Positions): ${:.2}", total_current_value);
    println!("   Realized P&L: ${:.2}", realized_pnl);
    println!("   Unrealized P&L: ${:.2}", unrealized_pnl);
    println!("   Total P&L: ${:.2}", total_pnl);
    println!("   P&L if All Closed: ${:.2} (if all positions closed at last price)\n", expected_pnl_if_closed);

    println!("🔍 Discrepancy Analysis:\n");
    
    // Check for potential issues
    let mut issues_found = false;
    
    // 1. Check if there are many skipped trades
    let skip_rate = (skipped_trades as f64 / all_trades.len() as f64) * 100.0;
    if skip_rate > 50.0 {
        println!("⚠️  High skip rate: {:.1}% of trades were skipped", skip_rate);
        println!("   This may indicate risk guard settings are too strict.\n");
        issues_found = true;
    }
    
    // 2. Check for failed trades
    if failed_trades > 0 {
        let fail_rate = (failed_trades as f64 / all_trades.len() as f64) * 100.0;
        println!("⚠️  Failed trades detected: {} ({:.1}%)", failed_trades, fail_rate);
        println!("   Common causes: insufficient balance, allowance issues, or API errors.\n");
        issues_found = true;
    }
    
    // 3. Check for positions with zero price (no recent trades)
    if positions_with_zero_price > 0 {
        println!("⚠️  {} position(s) have no recent price data (last_price = 0)", positions_with_zero_price);
        println!("   Current value calculation may be inaccurate. Check market status.\n");
        issues_found = true;
    }
    
    // 4. Check for large unrealized losses
    if unrealized_pnl < -10.0 {
        println!("⚠️  Significant unrealized losses: ${:.2}", unrealized_pnl);
        println!("   Consider reviewing open positions and market conditions.\n");
        issues_found = true;
    }
    
    // 5. Check for positions with very small shares (dust)
    let dust_positions: Vec<&Position> = positions.values()
        .filter(|p| p.total_shares > 0.001 && p.total_shares < 0.1 && (p.total_shares * p.last_price) < 0.10)
        .collect();
    if !dust_positions.is_empty() {
        println!("⚠️  Dust positions detected: {} position(s) with value < $0.10", dust_positions.len());
        println!("   These may be difficult to close and could accumulate fees.\n");
        issues_found = true;
    }
    
    // 6. Check for positions with negative cost basis (shouldn't happen)
    let negative_cost: Vec<&Position> = positions.values()
        .filter(|p| p.total_shares > 0.001 && p.total_cost < 0.0)
        .collect();
    if !negative_cost.is_empty() {
        println!("⚠️  Negative cost basis detected: {} position(s)", negative_cost.len());
        println!("   This may indicate data inconsistency in the CSV file.\n");
        issues_found = true;
    }
    
    if !issues_found {
        println!("✅ No major discrepancies detected.");
        println!("   Your trading data appears consistent.\n");
    }

    println!("💡 Tips:");
    println!("   - Realized P&L: Profit/loss from closed positions");
    println!("   - Unrealized P&L: Current value vs cost basis of open positions");
    println!("   - Use 'cargo run --release wallet check-positions-detailed' for position details");
    println!("   - Use 'cargo run --release wallet check-recent-activity' to review recent trades\n");
    
    Ok(())
}

async fn find_my_eoa() -> Result<()> {
    dotenvy::dotenv().ok();
    
    println!("🕵️ Finding EOA Wallet");
    println!("====================\n");

    // Get private key from environment
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set. Add it to your .env file."))?;
    
    // Parse private key to get signer and address
    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    
    let eoa_address = signer.address();
    
    println!("📝 EOA Wallet Address: {}", eoa_address);
    println!("   Type: Externally Owned Account (EOA)");
    println!("   Network: Polygon (Chain ID: 137)\n");
    
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
    
    println!("🌐 Using RPC: {}", if rpc_url.contains("alchemy") { "Alchemy" } else if rpc_url.contains("chainstack") { "Chainstack" } else { "Public RPC" });
    
    // Get provider
    let provider = ProviderBuilder::new()
        .wallet(signer.clone())
        .connect_http(rpc_url.parse()?);
    
    // Get balance and transaction count
    let client = reqwest::Client::new();
    
    // Get MATIC balance
    let balance_result = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBalance",
            "params": [format!("{:#x}", eoa_address), "latest"],
            "id": 1
        }))
        .send()
        .await?;
    
    let balance_json: serde_json::Value = balance_result.json().await?;
    let matic_balance_hex = balance_json["result"].as_str().unwrap_or("0x0");
    let matic_balance = U256::from_str_radix(matic_balance_hex.strip_prefix("0x").unwrap_or(matic_balance_hex), 16)?;
    let matic_balance_formatted = format_units(matic_balance, 18);
    
    // Get transaction count (nonce)
    let tx_count_result = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionCount",
            "params": [format!("{:#x}", eoa_address), "latest"],
            "id": 2
        }))
        .send()
        .await?;
    
    let tx_count_json: serde_json::Value = tx_count_result.json().await?;
    let tx_count_hex = tx_count_json["result"].as_str().unwrap_or("0x0");
    let tx_count = U256::from_str_radix(tx_count_hex.strip_prefix("0x").unwrap_or(tx_count_hex), 16)?;
    
    // Get USDC balance
    let usdc_addr = Address::from_str(USDC_ADDRESS)?;
    let usdc = IERC20::new(usdc_addr, provider.clone());
    let usdc_balance = usdc.balanceOf(eoa_address).call().await?;
    let usdc_balance_formatted = format_units(usdc_balance, 6);
    
    println!("\n💰 Balance:");
    println!("   MATIC: {} MATIC", matic_balance_formatted);
    println!("   USDC: {} USDC", usdc_balance_formatted);
    
    println!("\n📊 Activity:");
    println!("   Transaction Count: {} (nonce)", tx_count);
    
    // Check if this is the funder address
    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| eoa_address);
    
    println!("\n🔗 Configuration:");
    if funder_address == eoa_address {
        println!("   ✅ This EOA is configured as FUNDER_ADDRESS");
        println!("   ✅ Signer and funder are the same (EOA mode)");
    } else {
        println!("   ⚠️  This EOA is NOT the configured FUNDER_ADDRESS");
        println!("   📝 FUNDER_ADDRESS: {}", funder_address);
        println!("   ℹ️  Using Gnosis Safe as funder, EOA as signer");
    }
    
    println!("\n💡 Tips:");
    println!("   - EOA (Externally Owned Account) is a regular wallet controlled by a private key");
    println!("   - This address can sign transactions directly");
    println!("   - If using Gnosis Safe, this EOA signs on behalf of the Safe");
    println!("   - View on PolygonScan: https://polygonscan.com/address/{}", eoa_address);
    println!();
    
    Ok(())
}

async fn find_gnosis_safe_proxy() -> Result<()> {
    dotenvy::dotenv().ok();
    
    println!("🏦 Finding Gnosis Safe Proxy");
    println!("============================\n");

    // Get private key and derive EOA
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set. Add it to your .env file."))?;
    
    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    
    let eoa_address = signer.address();
    
    // Get FUNDER_ADDRESS (Gnosis Safe)
    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| eoa_address);
    
    println!("📝 Wallet Information:");
    println!("   EOA Address (Signer): {}", eoa_address);
    println!("   Gnosis Safe (Funder): {}", funder_address);
    println!("   Network: Polygon (Chain ID: 137)\n");
    
    if funder_address == eoa_address {
        println!("⚠️  Configuration Notice:");
        println!("   Your FUNDER_ADDRESS is the same as your EOA address.");
        println!("   This means you're using EOA mode, not Gnosis Safe mode.\n");
        println!("   To use a Gnosis Safe as proxy wallet:");
        println!("   1. Create or use an existing Gnosis Safe on Polygon");
        println!("   2. Add your EOA as a signer/owner of the Safe");
        println!("   3. Set FUNDER_ADDRESS in .env to your Gnosis Safe address");
        println!("   4. Ensure the Safe has USDC and MATIC for trading\n");
    } else {
        println!("✅ Gnosis Safe Configuration Detected:");
        println!("   Your EOA ({}) signs on behalf of the Gnosis Safe ({})", eoa_address, funder_address);
        println!("   All orders will be placed from the Gnosis Safe address\n");
        
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
        
        // Get provider
        let provider = ProviderBuilder::new()
            .wallet(signer.clone())
            .connect_http(rpc_url.parse()?);
        
        let client = reqwest::Client::new();
        
        // Get Gnosis Safe balance
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
        let matic_balance_formatted = format_units(matic_balance, 18);
        
        // Get USDC balance
        let usdc_addr = Address::from_str(USDC_ADDRESS)?;
        let usdc = IERC20::new(usdc_addr, provider.clone());
        let usdc_balance = usdc.balanceOf(funder_address).call().await?;
        let usdc_balance_formatted = format_units(usdc_balance, 6);
        
        println!("💰 Gnosis Safe Balances:");
        println!("   MATIC: {} MATIC", matic_balance_formatted);
        println!("   USDC: {} USDC", usdc_balance_formatted);
        
        // Check if EOA is owner of Safe (basic check via code)
        println!("\n🔐 Security:");
        println!("   ✅ EOA can sign transactions on behalf of Safe");
        println!("   ⚠️  Verify EOA is an owner/signer in your Gnosis Safe");
        println!("   📝 View Safe: https://app.safe.global/polygon:{}/", funder_address);
        
        println!("\n💡 How It Works:");
        println!("   1. EOA ({}) signs orders using PRIVATE_KEY", eoa_address);
        println!("   2. Orders are placed from Gnosis Safe ({})", funder_address);
        println!("   3. Safe holds all funds (USDC, MATIC)");
        println!("   4. Safe provides multi-sig security and recovery options");
        
        println!("\n🔗 Links:");
        println!("   PolygonScan (Safe): https://polygonscan.com/address/{}", funder_address);
        println!("   PolygonScan (EOA): https://polygonscan.com/address/{}", eoa_address);
        println!("   Gnosis Safe App: https://app.safe.global/polygon:{}/", funder_address);
    }
    
    println!();
    Ok(())
}

async fn sell_large_positions() -> Result<()> {
    dotenvy::dotenv().ok();
    
    println!("📊 Sell Large Positions");
    println!("======================\n");

    // Load wallet configuration
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set"))?;
    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    
    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| signer.address());

    println!("📝 Wallet Address: {}", funder_address);
    if funder_address != signer.address() {
        println!("   ℹ️  Using Gnosis Safe as funder\n");
    } else {
        println!();
    }

    if !Path::new(CSV_FILE).exists() {
        println!("❌ No trading history found ({} not found)", CSV_FILE);
        println!("   The bot needs to run and make trades first.\n");
        return Ok(());
    }

    // Define threshold for "large" positions (default: $50 USD value)
    let large_position_threshold = 50.0; // USD
    
    let csv_content = fs::read_to_string(CSV_FILE)?;
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
    
    let mut positions: std::collections::HashMap<String, Position> = std::collections::HashMap::new();

    for result in reader.deserialize::<CsvRow>() {
        if let Ok(row) = result {
            // Skip trades that were explicitly skipped by risk guards
            if let Some(ref status) = row.order_status {
                if status.contains("SKIPPED") {
                    continue;
                }
            }

            if let (Some(token_id), Some(direction), Some(shares_str), Some(price_str), Some(usd_value_str)) = (
                row.clob_asset_id,
                row.direction,
                row.shares,
                row.price_per_share,
                row.usd_value,
            ) {
                let shares: f64 = shares_str.parse().unwrap_or(0.0);
                let price: f64 = price_str.parse().unwrap_or(0.0);
                let usd_value: f64 = usd_value_str.parse().unwrap_or(0.0);

                let position = positions.entry(token_id.clone()).or_insert_with(Position::default);
                position.token_id = token_id;
                position.last_price = price; // Always update with the last known price

                if direction.contains("BUY") {
                    position.total_shares += shares;
                    position.total_cost += usd_value;
                    position.buy_count += 1;
                } else if direction.contains("SELL") {
                    position.total_shares -= shares;
                    position.total_cost -= usd_value; // Reduce cost basis on sell
                    position.sell_count += 1;
                }
            }
        }
    }

    // Filter for large positions
    let mut large_positions: Vec<Position> = positions.into_iter()
        .filter_map(|(_, pos)| {
            // Only consider positions with meaningful shares
            if pos.total_shares > 0.001 {
                // Calculate current value
                let price = if pos.last_price > 0.0 {
                    pos.last_price
                } else {
                    // Fallback to average price
                    if pos.total_shares > 0.0 {
                        pos.total_cost / pos.total_shares
                    } else {
                        0.0
                    }
                };
                let current_value = pos.total_shares * price;
                
                // Check if position is "large" (value >= threshold)
                if current_value >= large_position_threshold {
                    Some(pos)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    if large_positions.is_empty() {
        println!("✅ No large positions found.");
        println!("   Threshold: ${:.2} USD", large_position_threshold);
        println!("   All positions are below this threshold.\n");
        return Ok(());
    }

    // Sort by current value descending
    large_positions.sort_by(|a, b| {
        let a_value = a.total_shares * if a.last_price > 0.0 { a.last_price } else { a.total_cost / a.total_shares.max(0.001) };
        let b_value = b.total_shares * if b.last_price > 0.0 { b.last_price } else { b.total_cost / b.total_shares.max(0.001) };
        b_value.partial_cmp(&a_value).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("🔍 Found {} large position(s) (value >= ${:.2} USD):\n", large_positions.len(), large_position_threshold);
    println!("{:-<120}", "");
    println!("{:<20} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12}", 
             "Token ID", "Shares", "Avg Price", "Cost Basis", "Last Price", "Current Val", "P&L");
    println!("{:-<120}", "");

    let mut total_value = 0.0;
    let mut total_cost = 0.0;

    for pos in &large_positions {
        let avg_price = if pos.buy_count > 0 { pos.total_cost / pos.total_shares } else { 0.0 };
        let price = if pos.last_price > 0.0 {
            pos.last_price
        } else {
            avg_price
        };
        let current_value = pos.total_shares * price;
        let pnl = current_value - pos.total_cost;
        let pnl_percent = if pos.total_cost > 0.001 { (pnl / pos.total_cost) * 100.0 } else { 0.0 };

        total_value += current_value;
        total_cost += pos.total_cost;

        println!("{:<20} {:<12.6} {:<12.4} {:<12.2} {:<12.4} {:<12.2} {:<+12.2} ({:<+6.1}%)",
                 if pos.token_id.len() > 17 { format!("{}...", &pos.token_id[..17]) } else { pos.token_id.clone() },
                 pos.total_shares,
                 avg_price,
                 pos.total_cost,
                 price,
                 current_value,
                 pnl,
                 pnl_percent);
    }
    println!("{:-<120}", "");
    let total_pnl = total_value - total_cost;
    let total_pnl_percent = if total_cost > 0.001 { (total_pnl / total_cost) * 100.0 } else { 0.0 };
    println!("{:<20} {:<12} {:<12} {:<12.2} {:<12} {:<12.2} {:<+12.2} ({:<+6.1}%)",
             "TOTAL", "", "", total_cost, "", total_value, total_pnl, total_pnl_percent);
    println!("{:-<120}", "");

    println!("\n⚠️  TODO: Implement CLOB sell order logic");
    println!("   Required steps:");
    println!("   1. For each large position:");
    println!("      - Fetch current market order book for token_id");
    println!("      - Determine optimal sell price (market or limit)");
    println!("      - Build sell order using CLOB client");
    println!("      - Sign order with appropriate signature type (EOA/GnosisSafe)");
    println!("      - Submit order via CLOB API");
    println!("      - Track order execution status");
    println!("   2. Handle errors (insufficient balance, market closed, etc.)");
    println!("   3. Log results to CSV file");
    println!("\n   Note: This requires full CLOB client implementation.");
    println!("   See: src/orders.rs for existing sell_order() function reference.\n");

    println!("💡 Tips:");
    println!("   - Large positions are defined as positions with value >= ${:.2} USD", large_position_threshold);
    println!("   - Consider market conditions before selling large positions");
    println!("   - Use 'cargo run --release wallet check-positions-detailed' to see all positions");
    println!("   - Use 'cargo run --release position manual-sell <market> <outcome> <amount>' for manual sells\n");
    
    Ok(())
}

async fn close_stale_positions() -> Result<()> {
    dotenvy::dotenv().ok();
    
    println!("🧹 Close Stale Positions");
    println!("========================\n");

    // Load wallet configuration
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set"))?;
    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    
    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| signer.address());

    println!("📝 Wallet Address: {}", funder_address);
    if funder_address != signer.address() {
        println!("   ℹ️  Using Gnosis Safe as funder\n");
    } else {
        println!();
    }

    if !Path::new(CSV_FILE).exists() {
        println!("❌ No trading history found ({} not found)", CSV_FILE);
        println!("   The bot needs to run and make trades first.\n");
        return Ok(());
    }

    // Define threshold for "stale" positions (default: 30 days old)
    let stale_days_threshold = 30;
    
    let csv_content = fs::read_to_string(CSV_FILE)?;
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
    
    // Track positions with their last trade timestamp
    let mut positions: std::collections::HashMap<String, (Position, Option<String>)> = std::collections::HashMap::new();

    for result in reader.deserialize::<CsvRow>() {
        if let Ok(row) = result {
            // Skip trades that were explicitly skipped by risk guards
            if let Some(ref status) = row.order_status {
                if status.contains("SKIPPED") {
                    continue;
                }
            }

            if let (Some(token_id), Some(direction), Some(shares_str), Some(price_str), Some(usd_value_str)) = (
                row.clob_asset_id,
                row.direction,
                row.shares,
                row.price_per_share,
                row.usd_value,
            ) {
                let shares: f64 = shares_str.parse().unwrap_or(0.0);
                let price: f64 = price_str.parse().unwrap_or(0.0);
                let usd_value: f64 = usd_value_str.parse().unwrap_or(0.0);

                let entry = positions.entry(token_id.clone()).or_insert_with(|| {
                    (Position {
                        token_id: token_id.clone(),
                        ..Default::default()
                    }, None)
                });
                
                let (position, last_timestamp) = entry;
                position.token_id = token_id;
                position.last_price = price; // Always update with the last known price

                if direction.contains("BUY") {
                    position.total_shares += shares;
                    position.total_cost += usd_value;
                    position.buy_count += 1;
                } else if direction.contains("SELL") {
                    position.total_shares -= shares;
                    position.total_cost -= usd_value; // Reduce cost basis on sell
                    position.sell_count += 1;
                }
                
                // Update last timestamp if available
                if let Some(ref ts) = row.timestamp {
                    *last_timestamp = Some(ts.clone());
                }
            }
        }
    }

    // Get current time
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Filter for stale positions and track all positions with ages
    let mut stale_positions: Vec<(Position, Option<String>, u64)> = Vec::new();
    let mut all_positions_with_age: Vec<(Position, Option<String>, Option<u64>)> = Vec::new();
    let mut positions_without_timestamp = 0;

    for (_, (pos, last_ts)) in positions.into_iter() {
        // Only consider positions with meaningful shares
        if pos.total_shares > 0.001 {
            let mut age_days: Option<u64> = None;
            
            // Parse timestamp and calculate age
            if let Some(ref ts_str) = last_ts {
                // Try to parse timestamp (format: "2026-01-16 23:06:31.824" or similar)
                if let Ok(parsed_time) = chrono::NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S%.f") {
                    let trade_time = parsed_time.and_utc().timestamp() as u64;
                    age_days = Some((now - trade_time) / 86400); // seconds to days
                } else if let Ok(parsed_time) = chrono::DateTime::parse_from_rfc3339(ts_str) {
                    let trade_time = parsed_time.timestamp() as u64;
                    age_days = Some((now - trade_time) / 86400);
                }
            }
            
            if let Some(age) = age_days {
                all_positions_with_age.push((pos.clone(), last_ts.clone(), Some(age)));
                if age >= stale_days_threshold {
                    stale_positions.push((pos, last_ts.clone(), age));
                }
            } else {
                positions_without_timestamp += 1;
            }
        }
    }

    // Show summary of all positions
    println!("📊 Position Analysis:");
    println!("   Total open positions: {}", all_positions_with_age.len() + positions_without_timestamp);
    println!("   Positions with timestamps: {}", all_positions_with_age.len());
    if positions_without_timestamp > 0 {
        println!("   Positions without timestamps: {} (cannot determine age)", positions_without_timestamp);
    }
    println!();

    if stale_positions.is_empty() {
        println!("✅ No stale positions found (threshold: {} days old)", stale_days_threshold);
        
        if !all_positions_with_age.is_empty() {
            // Show positions that are close to being stale (within 5 days of threshold)
            let warning_threshold = stale_days_threshold.saturating_sub(5);
            let near_stale: Vec<_> = all_positions_with_age.iter()
                .filter_map(|(pos, _ts, age)| {
                    if let Some(age_val) = *age {
                        if age_val >= warning_threshold && age_val < stale_days_threshold {
                            Some((pos, age_val))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            
            if !near_stale.is_empty() {
                println!("\n⚠️  {} position(s) approaching stale threshold ({} - {} days old):", 
                         near_stale.len(), warning_threshold, stale_days_threshold - 1);
                for (pos, age) in near_stale.iter().take(5) {
                    println!("   - {}: {} days old ({} shares)", 
                             if pos.token_id.len() > 20 { format!("{}...", &pos.token_id[..20]) } else { pos.token_id.clone() },
                             age, pos.total_shares);
                }
                if near_stale.len() > 5 {
                    println!("   ... and {} more", near_stale.len() - 5);
                }
            }
            
            // Show youngest and oldest positions
            all_positions_with_age.sort_by(|a, b| {
                match (a.2, b.2) {
                    (Some(age_a), Some(age_b)) => age_b.cmp(&age_a), // Oldest first
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
            
            if all_positions_with_age.len() > 0 {
                println!("\n📅 Position Age Summary:");
                if let Some((oldest_pos, _, Some(oldest_age))) = all_positions_with_age.first() {
                    println!("   Oldest position: {} days old ({} shares)", oldest_age, oldest_pos.total_shares);
                }
                if let Some((youngest_pos, _, Some(youngest_age))) = all_positions_with_age.last() {
                    println!("   Youngest position: {} days old ({} shares)", youngest_age, youngest_pos.total_shares);
                }
            }
        }
        
        println!("\n💡 All positions have recent activity (within {} days).", stale_days_threshold);
        println!();
        return Ok(());
    }

    // Sort by age descending (oldest first)
    stale_positions.sort_by(|a, b| b.2.cmp(&a.2));

    println!("🔍 Found {} stale position(s) (last trade >= {} days ago):\n", stale_positions.len(), stale_days_threshold);
    println!("{:-<130}", "");
    println!("{:<20} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12}", 
             "Token ID", "Shares", "Avg Price", "Cost Basis", "Last Price", "Current Val", "Age (days)", "P&L");
    println!("{:-<130}", "");

    let mut total_value = 0.0;
    let mut total_cost = 0.0;

    for (pos, _last_ts, age_days) in &stale_positions {
        let avg_price = if pos.buy_count > 0 { pos.total_cost / pos.total_shares } else { 0.0 };
        let price = if pos.last_price > 0.0 {
            pos.last_price
        } else {
            avg_price
        };
        let current_value = pos.total_shares * price;
        let pnl = current_value - pos.total_cost;
        let pnl_percent = if pos.total_cost > 0.001 { (pnl / pos.total_cost) * 100.0 } else { 0.0 };

        total_value += current_value;
        total_cost += pos.total_cost;

        println!("{:<20} {:<12.6} {:<12.4} {:<12.2} {:<12.4} {:<12.2} {:<12} {:<+12.2} ({:<+6.1}%)",
                 if pos.token_id.len() > 17 { format!("{}...", &pos.token_id[..17]) } else { pos.token_id.clone() },
                 pos.total_shares,
                 avg_price,
                 pos.total_cost,
                 price,
                 current_value,
                 format!("{} days", age_days),
                 pnl,
                 pnl_percent);
    }
    println!("{:-<130}", "");
    let total_pnl = total_value - total_cost;
    let total_pnl_percent = if total_cost > 0.001 { (total_pnl / total_cost) * 100.0 } else { 0.0 };
    println!("{:<20} {:<12} {:<12} {:<12.2} {:<12} {:<12.2} {:<12} {:<+12.2} ({:<+6.1}%)",
             "TOTAL", "", "", total_cost, "", total_value, "", total_pnl, total_pnl_percent);
    println!("{:-<130}", "");

    println!("\n⚠️  TODO: Implement CLOB sell order logic for stale positions");
    println!("   Required steps:");
    println!("   1. For each stale position:");
    println!("      - Fetch current market order book for token_id");
    println!("      - Check if market is still active/live");
    println!("      - Determine optimal sell price (market or limit)");
    println!("      - Build sell order using CLOB client");
    println!("      - Sign order with appropriate signature type (EOA/GnosisSafe)");
    println!("      - Submit order via CLOB API");
    println!("      - Track order execution status");
    println!("   2. Handle errors (insufficient balance, market closed/resolved, etc.)");
    println!("   3. Log results to CSV file");
    println!("\n   Note: This requires full CLOB client implementation.");
    println!("   See: src/orders.rs for existing sell_order() function reference.\n");

    println!("💡 Tips:");
    println!("   - Stale positions are defined as positions with last trade >= {} days ago", stale_days_threshold);
    println!("   - Consider market status before closing stale positions (may be resolved/closed)");
    println!("   - Use 'cargo run --release wallet check-positions-detailed' to see all positions");
    println!("   - Use 'cargo run --release position manual-sell <market> <outcome> <amount>' for manual sells\n");
    
    Ok(())
}

async fn close_resolved_positions() -> Result<()> {
    dotenvy::dotenv().ok();
    
    println!("✅ Close Resolved Positions");
    println!("==========================\n");

    // Load wallet configuration
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set"))?;
    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    
    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| signer.address());

    println!("📝 Wallet Address: {}", funder_address);
    if funder_address != signer.address() {
        println!("   ℹ️  Using Gnosis Safe as funder\n");
    } else {
        println!();
    }

    if !Path::new(CSV_FILE).exists() {
        println!("❌ No trading history found ({} not found)", CSV_FILE);
        println!("   The bot needs to run and make trades first.\n");
        return Ok(());
    }
    
    let csv_content = fs::read_to_string(CSV_FILE)?;
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
    
    // Track positions by token_id
    let mut positions: std::collections::HashMap<String, Position> = std::collections::HashMap::new();
    // Track unique token IDs for market resolution checking
    let mut token_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for result in reader.deserialize::<CsvRow>() {
        if let Ok(row) = result {
            // Skip trades that were explicitly skipped by risk guards
            if let Some(ref status) = row.order_status {
                if status.contains("SKIPPED") {
                    continue;
                }
            }

            if let (Some(token_id), Some(direction), Some(shares_str), Some(price_str), Some(usd_value_str)) = (
                row.clob_asset_id,
                row.direction,
                row.shares,
                row.price_per_share,
                row.usd_value,
            ) {
                token_ids.insert(token_id.clone());
                
                let shares: f64 = shares_str.parse().unwrap_or(0.0);
                let price: f64 = price_str.parse().unwrap_or(0.0);
                let usd_value: f64 = usd_value_str.parse().unwrap_or(0.0);

                let position = positions.entry(token_id.clone()).or_insert_with(Position::default);
                position.token_id = token_id;
                position.last_price = price; // Always update with the last known price

                if direction.contains("BUY") {
                    position.total_shares += shares;
                    position.total_cost += usd_value;
                    position.buy_count += 1;
                } else if direction.contains("SELL") {
                    position.total_shares -= shares;
                    position.total_cost -= usd_value; // Reduce cost basis on sell
                    position.sell_count += 1;
                }
            }
        }
    }

    // Filter to only open positions
    let open_positions: Vec<Position> = positions.into_iter()
        .filter_map(|(_, pos)| {
            if pos.total_shares > 0.001 {
                Some(pos)
            } else {
                None
            }
        })
        .collect();

    if open_positions.is_empty() {
        println!("✅ No open positions found.");
        println!("   All positions have been closed.\n");
        return Ok(());
    }

    println!("📊 Found {} open position(s) across {} unique token(s):\n", open_positions.len(), token_ids.len());
    
    println!("⚠️  TODO: Implement market resolution checking via CLOB API");
    println!("   Required steps:");
    println!("   1. For each unique token_id:");
    println!("      - Query CLOB API to get market information");
    println!("      - Check market status (is_live, is_resolved, resolution_date, etc.)");
    println!("      - Identify which markets have resolved");
    println!("   2. Filter positions to only those in resolved markets");
    println!("   3. For each resolved position:");
    println!("      - Determine winning outcome (if position is in winning outcome)");
    println!("      - Calculate potential redemption value");
    println!("      - Option 1: Build sell order to close position");
    println!("      - Option 2: Call ConditionalTokens.redeemPositions() to collect winnings");
    println!("      - Submit transaction via CLOB client or direct contract call");
    println!("      - Track execution status");
    println!("   4. Handle errors (market not resolved, position already redeemed, etc.)");
    println!("   5. Log results to CSV file");
    println!("\n   Note: This requires:");
    println!("   - CLOB API integration for market status queries");
    println!("   - ConditionalTokens contract interaction for redemption");
    println!("   - See: src/orders.rs for existing order functions");
    println!("   - See: market_cache.rs for market data structures\n");

    // Show all open positions (user can manually check which are resolved)
    println!("📋 All Open Positions (check market status manually if needed):\n");
    println!("{:-<120}", "");
    println!("{:<20} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12}", 
             "Token ID", "Shares", "Avg Price", "Cost Basis", "Last Price", "Current Val", "P&L");
    println!("{:-<120}", "");

    let mut total_value = 0.0;
    let mut total_cost = 0.0;

    for pos in &open_positions {
        let avg_price = if pos.buy_count > 0 { pos.total_cost / pos.total_shares } else { 0.0 };
        let price = if pos.last_price > 0.0 {
            pos.last_price
        } else {
            avg_price
        };
        let current_value = pos.total_shares * price;
        let pnl = current_value - pos.total_cost;
        let pnl_percent = if pos.total_cost > 0.001 { (pnl / pos.total_cost) * 100.0 } else { 0.0 };

        total_value += current_value;
        total_cost += pos.total_cost;

        println!("{:<20} {:<12.6} {:<12.4} {:<12.2} {:<12.4} {:<12.2} {:<+12.2} ({:<+6.1}%)",
                 if pos.token_id.len() > 17 { format!("{}...", &pos.token_id[..17]) } else { pos.token_id.clone() },
                 pos.total_shares,
                 avg_price,
                 pos.total_cost,
                 price,
                 current_value,
                 pnl,
                 pnl_percent);
    }
    println!("{:-<120}", "");
    let total_pnl = total_value - total_cost;
    let total_pnl_percent = if total_cost > 0.001 { (total_pnl / total_cost) * 100.0 } else { 0.0 };
    println!("{:<20} {:<12} {:<12} {:<12.2} {:<12} {:<12.2} {:<+12.2} ({:<+6.1}%)",
             "TOTAL", "", "", total_cost, "", total_value, total_pnl, total_pnl_percent);
    println!("{:-<120}", "");

    println!("\n💡 Tips:");
    println!("   - Resolved markets are markets where the outcome has been determined");
    println!("   - Positions in winning outcomes can be redeemed for full value (1.0 per share)");
    println!("   - Positions in losing outcomes are worth 0 and should be closed/redeemed");
    println!("   - Use 'cargo run --release wallet check-positions-detailed' to see all positions");
    println!("   - Use 'cargo run --release position redeem-resolved' to redeem winning positions");
    println!("   - Check Polymarket directly to see which markets have resolved\n");
    
    Ok(())
}

async fn redeem_resolved_positions() -> Result<()> {
    dotenvy::dotenv().ok();
    
    println!("💵 Redeem Resolved Positions");
    println!("============================\n");

    // Load wallet configuration
    let private_key = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY environment variable not set"))?;
    let signer: PrivateKeySigner = private_key.parse()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    
    let funder_address = env::var("FUNDER_ADDRESS")
        .map(|addr| addr.trim().strip_prefix("0x").unwrap_or(&addr).to_string())
        .ok()
        .and_then(|addr| Address::from_str(&addr).ok())
        .unwrap_or_else(|| signer.address());

    println!("📝 Wallet Address: {}", funder_address);
    if funder_address != signer.address() {
        println!("   ℹ️  Using Gnosis Safe as funder\n");
    } else {
        println!();
    }

    if !Path::new(CSV_FILE).exists() {
        println!("❌ No trading history found ({} not found)", CSV_FILE);
        println!("   The bot needs to run and make trades first.\n");
        return Ok(());
    }
    
    let csv_content = fs::read_to_string(CSV_FILE)?;
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
    
    // Track positions by token_id
    let mut positions: std::collections::HashMap<String, Position> = std::collections::HashMap::new();
    // Track unique token IDs for market resolution checking
    let mut token_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for result in reader.deserialize::<CsvRow>() {
        if let Ok(row) = result {
            // Skip trades that were explicitly skipped by risk guards
            if let Some(ref status) = row.order_status {
                if status.contains("SKIPPED") {
                    continue;
                }
            }

            if let (Some(token_id), Some(direction), Some(shares_str), Some(price_str), Some(usd_value_str)) = (
                row.clob_asset_id,
                row.direction,
                row.shares,
                row.price_per_share,
                row.usd_value,
            ) {
                token_ids.insert(token_id.clone());
                
                let shares: f64 = shares_str.parse().unwrap_or(0.0);
                let price: f64 = price_str.parse().unwrap_or(0.0);
                let usd_value: f64 = usd_value_str.parse().unwrap_or(0.0);

                let position = positions.entry(token_id.clone()).or_insert_with(Position::default);
                position.token_id = token_id;
                position.last_price = price; // Always update with the last known price

                if direction.contains("BUY") {
                    position.total_shares += shares;
                    position.total_cost += usd_value;
                    position.buy_count += 1;
                } else if direction.contains("SELL") {
                    position.total_shares -= shares;
                    position.total_cost -= usd_value; // Reduce cost basis on sell
                    position.sell_count += 1;
                }
            }
        }
    }

    // Filter to only open positions
    let open_positions: Vec<Position> = positions.into_iter()
        .filter_map(|(_, pos)| {
            if pos.total_shares > 0.001 {
                Some(pos)
            } else {
                None
            }
        })
        .collect();

    if open_positions.is_empty() {
        println!("✅ No open positions found.");
        println!("   All positions have been closed.\n");
        return Ok(());
    }

    println!("📊 Found {} open position(s) across {} unique token(s):\n", open_positions.len(), token_ids.len());
    
    println!("⚠️  TODO: Implement market resolution checking and redemption via ConditionalTokens contract");
    println!("   Required steps:");
    println!("   1. For each unique token_id:");
    println!("      - Query CLOB API to get market information");
    println!("      - Check market status (is_live, is_resolved, resolution_date, etc.)");
    println!("      - Identify which markets have resolved");
    println!("      - Determine winning outcome(s) for each resolved market");
    println!("   2. For each position in a resolved market:");
    println!("      - Check if position is in a winning outcome");
    println!("      - Calculate redemption value (winning positions = 1.0 per share, losing = 0)");
    println!("      - Group positions by condition_id (markets can have multiple outcomes)");
    println!("   3. For each condition_id with winning positions:");
    println!("      - Build ConditionalTokens.redeemPositions() call:");
    println!("        redeemPositions(");
    println!("          conditionId: condition_id,");
    println!("          indexSets: [outcome_index],");
    println!("          account: funder_address");
    println!("        )");
    println!("      - Sign transaction with appropriate signer (EOA or Gnosis Safe)");
    println!("      - Submit transaction to blockchain");
    println!("      - Wait for transaction confirmation");
    println!("      - Track redemption status");
    println!("   4. Handle errors:");
    println!("      - Market not resolved yet");
    println!("      - Position already redeemed");
    println!("      - Insufficient gas");
    println!("      - Contract call failures");
    println!("   5. Log results to CSV file");
    println!("\n   Note: This requires:");
    println!("   - CLOB API integration for market status queries");
    println!("   - ConditionalTokens contract ABI and interaction");
    println!("   - Contract address: 0x4d97dcd97ec945f40cf65f87097ace5ea0476045 (Polygon)");
    println!("   - See: src/orders.rs for existing contract interaction patterns");
    println!("   - See: market_cache.rs for market data structures\n");

    // Show all open positions with potential redemption values
    println!("📋 All Open Positions (potential redemption candidates):\n");
    println!("{:-<130}", "");
    println!("{:<20} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12}", 
             "Token ID", "Shares", "Cost Basis", "Last Price", "Current Val", "Max Redeem", "P&L", "Status");
    println!("{:-<130}", "");

    let mut total_cost = 0.0;
    let mut total_current_value = 0.0;
    let mut total_max_redeem = 0.0;

    for pos in &open_positions {
        let price = if pos.last_price > 0.0 {
            pos.last_price
        } else {
            if pos.total_shares > 0.0 {
                pos.total_cost / pos.total_shares
            } else {
                0.0
            }
        };
        let current_value = pos.total_shares * price;
        let max_redeem_value = pos.total_shares * 1.0; // If winning, can redeem for 1.0 per share
        let pnl = current_value - pos.total_cost;
        let pnl_percent = if pos.total_cost > 0.001 { (pnl / pos.total_cost) * 100.0 } else { 0.0 };

        total_cost += pos.total_cost;
        total_current_value += current_value;
        total_max_redeem += max_redeem_value;

        println!("{:<20} {:<12.6} {:<12.2} {:<12.4} {:<12.2} {:<12.2} {:<+12.2} ({:<+6.1}%) {:<12}",
                 if pos.token_id.len() > 17 { format!("{}...", &pos.token_id[..17]) } else { pos.token_id.clone() },
                 pos.total_shares,
                 pos.total_cost,
                 price,
                 current_value,
                 max_redeem_value,
                 pnl,
                 pnl_percent,
                 "Unknown");
    }
    println!("{:-<130}", "");
    let total_pnl = total_current_value - total_cost;
    let total_pnl_percent = if total_cost > 0.001 { (total_pnl / total_cost) * 100.0 } else { 0.0 };
    println!("{:<20} {:<12} {:<12.2} {:<12} {:<12.2} {:<12.2} {:<+12.2} ({:<+6.1}%) {:<12}",
             "TOTAL", "", total_cost, "", total_current_value, total_max_redeem, total_pnl, total_pnl_percent, "");
    println!("{:-<130}", "");

    println!("\n💡 Redemption Information:");
    println!("   - Winning positions in resolved markets can be redeemed for 1.0 per share");
    println!("   - Losing positions in resolved markets are worth 0 (no redemption value)");
    println!("   - Redemption requires calling ConditionalTokens.redeemPositions() contract method");
    println!("   - For Gnosis Safe, redemption must be executed through Safe interface");
    println!("   - Use 'cargo run --release position close-resolved' to see all positions");
    println!("   - Check Polymarket directly to see which markets have resolved");
    println!("   - ConditionalTokens contract: 0x4d97dcd97ec945f40cf65f87097ace5ea0476045\n");
    
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

