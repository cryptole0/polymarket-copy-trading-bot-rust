//! Check market information utility
//! Run with: cargo run --release --bin check_market <token_id>
//!
//! Fetches and displays market information for a given token ID

use anyhow::{Result, anyhow};
use dotenvy::dotenv;
use std::env;
use reqwest::Client;

const CLOB_API_BASE: &str = "https://clob.polymarket.com";
const GAMMA_API_BASE: &str = "https://gamma-api.polymarket.com";

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --release --bin check_market <token_id>");
        eprintln!("\nExample:");
        eprintln!("  cargo run --release --bin check_market 54829853978330669429551251905778214074128014124609781186771015417529556703558");
        return Ok(());
    }

    let token_id = &args[1];
    println!("📊 Market Information Checker");
    println!("=============================\n");
    println!("Token ID: {}\n", token_id);

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // Fetch order book
    println!("📖 Fetching order book...");
    match fetch_order_book(&client, token_id).await {
        Ok(book) => {
            if let Some(best_bid) = book.best_bid {
                println!("   Best Bid: ${} @ {} shares", best_bid.0, best_bid.1);
            } else {
                println!("   Best Bid: No bids");
            }
            if let Some(best_ask) = book.best_ask {
                println!("   Best Ask: ${} @ {} shares", best_ask.0, best_ask.1);
            } else {
                println!("   Best Ask: No asks");
            }
            if let (Some(bid), Some(ask)) = (book.best_bid, book.best_ask) {
                let spread = ask.0 - bid.0;
                let spread_pct = (spread / bid.0) * 100.0;
                println!("   Spread: ${:.4} ({:.2}%)", spread, spread_pct);
            }
        }
        Err(e) => println!("   ❌ Failed to fetch order book: {}", e),
    }

    // Fetch market info from gamma API
    println!("\n📈 Fetching market info...");
    match fetch_market_info(&client, token_id).await {
        Ok(info) => {
            println!("   Market: {}", info.market);
            println!("   Outcome: {}", info.outcome);
            println!("   Question: {}", info.question);
            println!("   Condition ID: {}", info.condition_id);
            println!("   Live: {}", if info.is_live { "Yes" } else { "No" });
        }
        Err(e) => println!("   ❌ Failed to fetch market info: {}", e),
    }

    // Check cache info
    println!("\n💾 Checking cache...");
    pm_whale_follower::market_cache::init_caches();
    let caches = pm_whale_follower::market_cache::global_caches();
    
    if let Some(neg_risk) = pm_whale_follower::market_cache::is_neg_risk(token_id) {
        println!("   Neg Risk: {}", neg_risk);
    }
    if let Some(slug) = pm_whale_follower::market_cache::get_slug(token_id) {
        println!("   Slug: {}", slug);
    }
    if let Some(is_live) = pm_whale_follower::market_cache::get_is_live(token_id) {
        println!("   Live (cached): {}", if is_live { "Yes" } else { "No" });
    }

    let atp_buffer = pm_whale_follower::market_cache::get_atp_token_buffer(token_id);
    let ligue1_buffer = pm_whale_follower::market_cache::get_ligue1_token_buffer(token_id);
    if atp_buffer > 0.0 {
        println!("   ATP Buffer: {:.2}%", atp_buffer * 100.0);
    }
    if ligue1_buffer > 0.0 {
        println!("   Ligue1 Buffer: {:.2}%", ligue1_buffer * 100.0);
    }

    println!();
    Ok(())
}

struct OrderBook {
    best_bid: Option<(f64, f64)>,
    best_ask: Option<(f64, f64)>,
}

async fn fetch_order_book(client: &Client, token_id: &str) -> Result<OrderBook> {
    let url = format!("{}/book?token_id={}", CLOB_API_BASE, token_id);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow!("HTTP error: {}", resp.status()));
    }

    let book: serde_json::Value = resp.json().await?;

    let best_bid = book["bids"]
        .as_array()
        .and_then(|bids| {
            bids.iter()
                .filter_map(|b| {
                    let price: f64 = b["price"].as_str()?.parse().ok()?;
                    let size: f64 = b["size"].as_str()?.parse().ok()?;
                    Some((price, size))
                })
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        });

    let best_ask = book["asks"]
        .as_array()
        .and_then(|asks| {
            asks.iter()
                .filter_map(|a| {
                    let price: f64 = a["price"].as_str()?.parse().ok()?;
                    let size: f64 = a["size"].as_str()?.parse().ok()?;
                    Some((price, size))
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        });

    Ok(OrderBook { best_bid, best_ask })
}

struct MarketInfo {
    market: String,
    outcome: String,
    question: String,
    condition_id: String,
    is_live: bool,
}

async fn fetch_market_info(client: &Client, token_id: &str) -> Result<MarketInfo> {
    let url = format!("{}/markets?token_ids={}", GAMMA_API_BASE, token_id);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow!("HTTP error: {}", resp.status()));
    }

    let data: serde_json::Value = resp.json().await?;
    let markets = data.as_array().ok_or_else(|| anyhow!("Expected array"))?;
    
    if markets.is_empty() {
        return Err(anyhow!("No market found for token ID"));
    }

    let market = &markets[0];
    let token = market["tokens"]
        .as_array()
        .ok_or_else(|| anyhow!("Tokens field is not an array"))?
        .iter()
        .find(|t| t["token_id"].as_str() == Some(token_id))
        .ok_or_else(|| anyhow!("Token not found in market"))?;

    Ok(MarketInfo {
        market: market["question"].as_str().unwrap_or("Unknown").to_string(),
        outcome: token["outcome"].as_str().unwrap_or("Unknown").to_string(),
        question: market["question"].as_str().unwrap_or("Unknown").to_string(),
        condition_id: market["condition_id"].as_str().unwrap_or("Unknown").to_string(),
        is_live: market["active"].as_bool().unwrap_or(false),
    })
}

