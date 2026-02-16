# Polymarket Copy Trading Bot

> A bot that copies Polymarket whale trades in real time with your own risk limits and position sizing.

**Why this bot over Python or TypeScript bots:** Built in **Rust** for speed and reliability—no interpreter or GC pauses, so you get lower latency and more consistent execution. One compiled binary, minimal CPU/memory use, and strong typing so it keeps running without the runtime surprises common in scripted bots.

---


## Step-by-Step Trading Guide

Follow this guide from zero to live copy trading. Each step is designed so you can verify everything before risking real funds.

---

### Step 1 — Get Ready (Before You Trade)

| What you need | Where to get it |
|---------------|-----------------|
| **Polymarket account** | [polymarket.com](https://polymarket.com) |
| **Web3 wallet** (MetaMask or similar) | Export **private key** for a wallet you will use only for this bot |
| **RPC API key** | Free: [Alchemy](https://www.alchemy.com/) or [Chainstack](https://chainstack.com/) (Polygon network) |
| **Whale address** | The trader you want to copy (40-character hex; find via [Research commands](#55-research-commands)) |
| **Funds** | USDC on Polygon (e.g. 50–100 USDC to start) + a little MATIC for gas |

---

### Step 2 — Install the Bot

1. **Install Rust** (one-time):
   - **Windows:** [rustup.rs](https://rustup.rs/) → run installer → restart terminal.
   - **macOS/Linux:**  
     `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` then `source ~/.cargo/env`

2. **Clone and enter the project:**
   ```bash
   git clone <repository-url>
   cd rust
   ```

3. **Check versions:**
   ```bash
   rustc --version
   cargo --version
   ```

---

### Step 3 — Configure (One-Time Setup)

1. **Run the setup wizard** (creates and fills `.env`):
   ```bash
   cargo run --release setup setup
   ```

2. **When prompted, enter:**
   - **Target whale address** — The trader you’re copying
   - **Private key** — Your bot wallet’s key (64 hex chars, no `0x`)
   - **Funder address** — Same as wallet address for normal wallets; for Gnosis Safe, use the Safe address
   - **RPC API key** — From Alchemy or Chainstack (Polygon)
   - **Strategy** — `PERCENTAGE` (e.g. copy 10% of each trade), `FIXED` (e.g. $50 per trade), or `ADAPTIVE`
   - **Risk limits** — Max/min order size (USD), and optionally max position and daily volume

3. **Validate setup:**
   ```bash
   cargo run --release setup system-status
   ```
   Fix any errors (balance, connectivity, config) before continuing.

---

### Step 4 — Approve Tokens (Required for Real Trades)

The bot needs permission to spend your USDC and conditional tokens:

```bash
cargo run --release wallet check-allowance
```

- **EOA (MetaMask-style):** The bot can set allowances for you.
- **Gnosis Safe:** Follow the on-screen instructions to approve in the Safe UI.

---

### Step 5 — Test in Mock Mode (Strongly Recommended)

Run the bot **without** placing real orders:

1. In `.env` set:
   - `MOCK_TRADING=true`
   - `ENABLE_TRADING=true`

2. Start the bot:
   ```bash
   cargo run --release main run
   ```

3. Watch the logs: you’ll see which trades *would* have been placed. Let it run for a while to confirm behavior.

4. Stop with `Ctrl+C` when done testing.

---

### Step 6 — Go Live

1. In `.env` set:
   - `MOCK_TRADING=false`
   - `ENABLE_TRADING=true`

2. Start the bot:
   ```bash
   cargo run --release main run
   ```

3. The bot will:
   - Monitor the whale’s trades in real time
   - Size your orders (by your chosen strategy and limits)
   - Place copy trades on Polymarket
   - Log activity to CSV

4. Stop anytime with `Ctrl+C` (it will finish current work then exit).

---

### Step 7 — Monitor and Adjust

| Goal | Command |
|------|--------|
| Check balance & status | `cargo run --release setup system-status` |
| Recent activity | `cargo run --release wallet check-recent-activity` |
| Positions | `cargo run --release wallet check-positions-detailed` |
| P&L / stats | `cargo run --release wallet check-my-stats` |

To change risk or strategy: edit `.env` (e.g. `COPY_SIZE`, `MAX_ORDER_SIZE_USD`, `COPY_STRATEGY`), then restart the bot.

---

### Quick Reference — Before You Trade

- [ ] Wallet has USDC (and a little MATIC for gas) on Polygon  
- [ ] `.env` has correct `PRIVATE_KEY`, `FUNDER_ADDRESS`, `TARGET_WHALE_ADDRESS`, and RPC key  
- [ ] `cargo run --release setup system-status` passes  
- [ ] Token allowances set via `cargo run --release wallet check-allowance`  
- [ ] Tested with `MOCK_TRADING=true`  
- [ ] Using a dedicated wallet (not your main one)

---

## 0. 📁 Project Structure

> **New to the codebase?** See [docs/PROJECT_STRUCTURE.md](docs/PROJECT_STRUCTURE.md) for folder layout, binaries (setup, wallet, monitor, tools), and where to customize.

---

## 1. Quick Start

**Minimal path** (after Rust is installed and repo cloned):

```bash
# 1. Configure (interactive wizard)
cargo run --release setup setup

# 2. Validate
cargo run --release setup system-status

# 3. Approve tokens
cargo run --release wallet check-allowance

# 4. Run bot (use MOCK_TRADING=true in .env first to test)
cargo run --release main run
```

> **💡 Windows:** Install Rust from [rustup.rs](https://rustup.rs/), then use PowerShell or CMD for the commands above.

---

## 2. 📦 Installation

### 2.1 Prerequisites

- **Rust** 1.70 or later
- **Polygon network** access (for Polygon mainnet trading)
- **RPC provider** API key (Alchemy or Chainstack recommended)

### 2.2 Install Rust

**Windows:**
1. Download and run the installer from https://rustup.rs/
2. Follow the installation wizard
3. Restart your terminal/PowerShell

**macOS/Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2.3 Verify Installation

```bash
rustc --version
cargo --version
```

### 2.4 Clone Repository

```bash
git clone <repository-url>
cd rust
```

---

## 3. ⚙️ Configuration

### 3.1 Quick Setup Wizard (Recommended)

The easiest way to configure the bot is using the interactive setup wizard:

```bash
cargo run --release setup setup
```

The wizard will guide you through:

1. **Target whale address** - The trader you want to copy
2. **Wallet configuration** - Your private key and funder address
3. **RPC provider** - Alchemy API key
4. **Trading strategy** - PERCENTAGE, FIXED, or ADAPTIVE
5. **Risk limits** - Max/min order sizes, position caps, daily volume limits
6. **Multipliers** - Single and tiered multipliers

The wizard automatically:
- ✅ Creates your `.env` file
- ✅ Backs up existing `.env` if present
- ✅ Preserves important configuration sections

### 3.2 Manual Configuration

If you prefer manual setup, create a `.env` file from the example:

```bash
# Windows (PowerShell)
Copy-Item .env.example .env

# macOS/Linux
cp .env.example .env
```

### 3.3 Required Settings

Edit `.env` and set the following **required** variables:

```env
# Wallet Configuration
PRIVATE_KEY=your_private_key_here              # 64 hex chars, no 0x prefix
FUNDER_ADDRESS=your_funder_address_here        # 40 hex chars (Gnosis Safe or EOA)

# Trading Configuration
TARGET_WHALE_ADDRESS=target_whale_address      # 40 hex chars, no 0x prefix

# RPC Provider (choose ONE)
ALCHEMY_API_KEY=your_alchemy_api_key           # Recommended
# OR
CHAINSTACK_API_KEY=your_chainstack_api_key     # Alternative

# Trading Strategy
COPY_STRATEGY=PERCENTAGE                        # PERCENTAGE, FIXED, or ADAPTIVE
COPY_SIZE=10.0                                  # % for PERCENTAGE, $ for FIXED, base % for ADAPTIVE
TRADE_MULTIPLIER=1.0                            # Single multiplier (1.0 = normal)

# Risk Limits
MAX_ORDER_SIZE_USD=100.0                       # Maximum order size
MIN_ORDER_SIZE_USD=1.0                          # Minimum order size
# MAX_POSITION_SIZE_USD=500.0                   # Optional: Max position per market
# MAX_DAILY_VOLUME_USD=1000.0                  # Optional: Max daily volume

# Trading Flags
ENABLE_TRADING=true                             # Enable trading (false = no trades)
MOCK_TRADING=false                              # Mock mode (true = test mode, no real trades)
```

> **⚠️ Important for Gnosis Safe:**
> - `PRIVATE_KEY` = Private key that can sign on behalf of your Gnosis Safe
> - `FUNDER_ADDRESS` = Your Gnosis Safe address (proxy wallet)
> - These addresses will be **different**!
>
> **For regular EOA wallets:**
> - `PRIVATE_KEY` = Your wallet's private key
> - `FUNDER_ADDRESS` = Same address as the private key

### 3.4 Trading Strategies

The bot supports three trading strategies:

#### **PERCENTAGE** (Recommended)
- Copies a fixed percentage of the trader's order size
- Example: `COPY_SIZE=10.0` = 10% of trader's order
- Best for: Consistent risk exposure relative to trader

#### **FIXED**
- Always uses a fixed dollar amount per trade
- Example: `COPY_SIZE=50.0` = $50 per trade
- Best for: Fixed risk per trade, budget-conscious trading

#### **ADAPTIVE**
- Dynamically adjusts percentage based on trade size
- Higher % for small trades, lower % for large trades
- Requires: `ADAPTIVE_MIN_PERCENT`, `ADAPTIVE_MAX_PERCENT`, `ADAPTIVE_THRESHOLD_USD`
- Best for: Balancing risk across different trade sizes


### 3.5 Optional Settings

- Tiered multipliers
- Adaptive strategy parameters
- Risk management (circuit breakers)
- Advanced features

### 3.6 Validate Configuration

Before running, validate your configuration:

```bash
cargo run --release setup system-status
```

This checks:
- ✅ Configuration format
- ✅ Wallet balance
- ✅ Network connectivity
- ✅ Token approvals
- ✅ Trading strategy settings
- ✅ Risk limits

---

## 4.  Running the Bot

### 4.1 First-Time Setup

**1. Approve Tokens for Trading:**

```bash
cargo run --release wallet check-allowance
```

- For **Gnosis Safe wallets**, this will show manual approval instructions. Follow them to approve tokens through the Safe interface.
- For **EOA wallets**, this will auto-approve tokens.

**2. Test in Mock Mode (Recommended):**

Set `MOCK_TRADING=true` in `.env`, then:

```bash
cargo run --release main run
```

The bot will show what trades it would make without executing them.

**3. Start Live Trading:**

Set `ENABLE_TRADING=true` and `MOCK_TRADING=false` in `.env`, then:

```bash
cargo run --release main run
```

### 4.2 Running Options

**Using the unified CLI (recommended):**
```bash
cargo run --release <group> <command>
```

**Using individual binaries:**
```bash
cargo run --release --bin pm_bot       # Start bot
cargo run --release --bin validate_setup
cargo run --release --bin approve_tokens
```

**Helper scripts:**
```bash
# Linux/macOS
./run.sh

# Windows
run.bat
```

### 4.3 Stopping the Bot

Press `Ctrl+C` for graceful shutdown. The bot will finish current operations before exiting.

---

## 5. 💻 CLI Commands

The bot uses a unified CLI structure. Get help at any level:

```bash
cargo run --release -- --help              # Main help (all commands)
cargo run --release -- setup --help       # Setup commands help
cargo run --release -- wallet --help      # Wallet commands help
```

> **Note:** Use `--` to separate Cargo arguments from binary arguments. The `--help` flag should come after the command group.

### 5.1 Setup Commands

```bash
cargo run --release setup setup          # Interactive setup wizard (creates .env file)
cargo run --release setup system-status  # Validate config, check balances, connectivity, show strategy
cargo run --release setup help           # Print all available commands
```

**Setup Wizard Features:**
- ✅ Guides you through all configuration steps
- ✅ Validates inputs (addresses, keys, etc.)
- ✅ Backs up existing `.env` file automatically
- ✅ Shows current strategy and risk limits in system-status

### 5.2 Main Bot

```bash
cargo run --release main run             # Start the copy trading bot
```

### 5.3 Wallet Commands

```bash
cargo run --release wallet check-proxy-wallet         # Check Gnosis Safe balance/positions
cargo run --release wallet check-both-wallets <a1> <a2>  # Compare two wallets
cargo run --release wallet check-my-stats             # View wallet statistics
cargo run --release wallet check-recent-activity      # View recent trades
cargo run --release wallet check-positions-detailed   # View detailed positions
cargo run --release wallet check-pnl-discrepancy      # Analyze P&L discrepancies
cargo run --release wallet verify-allowance           # Verify token allowance
cargo run --release wallet check-allowance            # Check and set allowance
cargo run --release wallet set-token-allowance        # Set ERC1155 allowance
cargo run --release wallet find-my-eoa                # Find EOA wallet
cargo run --release wallet find-gnosis-safe-proxy     # Find Gnosis Safe proxy
```

### 5.4 Position Commands

```bash
cargo run --release position manual-sell <market> <outcome> <amount>  # Manual sell
cargo run --release position sell-large        # Sell large positions
cargo run --release position close-stale       # Close old positions
cargo run --release position close-resolved    # Close resolved positions
cargo run --release position redeem-resolved   # Redeem resolved positions
```

### 5.5 Research Commands

```bash
cargo run --release research find-best-traders      # Find top performers
cargo run --release research find-low-risk-traders  # Find low-risk traders
cargo run --release research scan-best-traders      # Scan top traders
cargo run --release research scan-from-markets      # Scan from markets
```

### 5.6 Simulation Commands

```bash
cargo run --release simulation simulate-profitability [trader]  # Simulate profitability
cargo run --release simulation simulate-profitability-old [trader]  # Legacy simulation
cargo run --release simulation run [preset]         # Run batch simulations
cargo run --release simulation compare [mode]       # Compare results
cargo run --release simulation aggregate            # Aggregate results
cargo run --release simulation audit                # Audit algorithm
cargo run --release simulation fetch-historical [--force] [--days N]  # Fetch historical data
```


**Getting help:**
```bash
cargo run --release -- --help              # Show all commands
cargo run --release -- setup --help        # Show setup commands
cargo run --release -- wallet --help       # Show wallet commands
```

> **Note:** Use `--` to separate Cargo arguments from binary arguments. The `--help` flag should come after the command group.

---

## 6. How It Works

### 6.1 Overview

The bot monitors successful traders (whales) and automatically copies their trades with intelligent scaling and risk management.

**Trading Flow:**
1. **Monitors** blockchain events for trades from target whale (real-time via WebSocket)
2. **Analyzes** each trade (size, price, market conditions) using multi-layer risk checks
3. **Calculates** position size (2% default) and price (whale price + buffer)
4. **Executes** scaled copy trade with optimized order types (FAK/GTD)
5. **Retries** failed orders with intelligent resubmission (up to 4-5 attempts)
6. **Protects** with risk guards (circuit breakers) and safety features
7. **Logs** everything to CSV files for analysis

### 6.2 Strategy Highlights

- **Three Trading Strategies:** PERCENTAGE (default), FIXED, and ADAPTIVE
- **Flexible Position Sizing:** Configurable percentage or fixed dollar amounts
- **Multipliers:** Single multiplier for all trades, plus optional tiered multipliers
- **Risk Limits:** Max/min order size, max position size, max daily volume caps
- **Tiered Execution:** Different strategies for large (4000+), medium (2000-3999), and small (<2000) trades
- **Multi-Layer Risk Management:** 4 layers of safety checks prevent dangerous trades
- **Intelligent Pricing:** Price buffers optimize fill rates (higher for large trades, none for small)
- **Sport-Specific Adjustments:** Additional buffers for tennis and soccer markets
- **Price Precision:** Automatic rounding to 3 decimal places (0.001 tick size) for Polymarket compliance


---

## 7. Features

### 7.1 Core Features

- ✅ **Real-time trade copying** - WebSocket-based monitoring
- ✅ **Three trading strategies** - PERCENTAGE, FIXED, and ADAPTIVE
- ✅ **Intelligent position sizing** - Configurable percentage or fixed amounts
- ✅ **Multipliers** - Single and tiered multipliers for flexible scaling
- ✅ **Risk limits** - Max/min order size, position caps, daily volume limits
- ✅ **Interactive setup wizard** - Guided configuration with validation
- ✅ **Circuit breakers** - Multi-layer risk management
- ✅ **Automatic order resubmission** - Handles failures with intelligent retry logic
- ✅ **Market cache system** - Fast market data lookups
- ✅ **CSV logging** - Complete trade history
- ✅ **Live market detection** - Adjusts order types based on market status
- ✅ **Gnosis Safe support** - Full support for multi-sig wallets
- ✅ **Price precision** - Automatic rounding to Polymarket's 0.001 tick size
- ✅ **Smart error handling** - Stops retrying on insufficient balance/allowance errors

### 7.2 Advanced Features

- ✅ **Tiered execution** - Different strategies based on trade size
- ✅ **Order type optimization** - FAK for immediate fills, GTD for limit orders
- ✅ **Price buffers** - Dynamic buffers based on trade size and market type
- ✅ **Sport-specific buffers** - Additional buffers for ATP and Ligue 1 markets
- ✅ **Rate limit handling** - Automatic retries with exponential backoff
- ✅ **Cache refresh** - Automatic background cache updates


---

## 8.  Requirements

### 8.1 Required

1. **A Polymarket Account** - Sign up at https://polymarket.com
2. **A Web3 Wallet** - Supports both:
   - **Regular EOA wallets** (MetaMask, etc.) - PRIVATE_KEY and FUNDER_ADDRESS should match
   - **Gnosis Safe wallets** - PRIVATE_KEY (signer) and FUNDER_ADDRESS (Safe) will be different
3. **RPC Provider API Key** - Free tier from:
   - [Alchemy](https://www.alchemy.com/) (recommended)
   - [Chainstack](https://chainstack.com/) (alternative)
4. **The Whale Address** - The trader you want to copy (40-character hex address)
5. **Token Approvals** - Must approve USDC and Conditional Tokens (see wallet commands)

### 8.2 Recommended

- **Some Coding Knowledge** - Not required, but helpful for troubleshooting
- **Sufficient Funds** - The bot uses 2% of whale trade size by default (configurable)
  - Minimum: 50-100 USDC recommended
  - Gas fees: 0.01-0.1 MATIC recommended
- **Gnosis Safe Setup** - For enhanced security with multi-sig wallets

---

## 9. Security

### 9.1 Security Best Practices

> **IMPORTANT:**
> - **Never share your `PRIVATE_KEY`** with anyone
> - **Never commit your `.env` file** to git (it's already in `.gitignore`)
> - **Start with small amounts** to test
> - **Use `MOCK_TRADING=true` first** to verify everything works
> - **Use a separate wallet** for bot trading (not your main wallet)
> - **Use Gnosis Safe** for enhanced security (multi-sig wallets)

### 9.2 Wallet Security

**For EOA wallets:**
- Store private key securely (password manager recommended)
- Use a dedicated wallet for bot trading
- Monitor wallet activity regularly

**For Gnosis Safe:**
- Private key should belong to an authorized signer
- Set appropriate threshold (2-of-3, 3-of-5, etc.)
- Monitor Safe activity through Safe interface

---

## 10. Troubleshooting

| Issue | What to do |
|-------|------------|
| **"Insufficient balance"** | Add USDC (and MATIC for gas) to your bot wallet on Polygon. |
| **"Allowance" / approval errors** | Run `cargo run --release wallet check-allowance` and complete approvals (Safe: use Safe UI). |
| **No trades copying** | Confirm `TARGET_WHALE_ADDRESS` is correct and the whale is active. Check `ENABLE_TRADING=true` and `MOCK_TRADING=false` for live trading. |
| **Bot exits or connection errors** | Check RPC key (Alchemy/Chainstack), network, and `setup system-status`. |
| **Orders too big/small** | Adjust `COPY_SIZE`, `MAX_ORDER_SIZE_USD`, `MIN_ORDER_SIZE_USD` in `.env`. |
| **Wrong strategy** | Set `COPY_STRATEGY` to `PERCENTAGE`, `FIXED`, or `ADAPTIVE` and set `COPY_SIZE` accordingly. |

For more: run `cargo run --release -- --help` and `cargo run --release setup system-status` to validate config and connectivity.

---
