//! Refresh market cache utility
//! Run with: cargo run --release --bin refresh_cache
//!
//! Manually refreshes all market caches

use anyhow::Result;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    println!("🔄 Market Cache Refresher");
    println!("=========================\n");

    println!("Loading caches...");
    pm_whale_follower::market_cache::init_caches();

    println!("Refreshing caches...");
    let result = pm_whale_follower::market_cache::refresh_caches();

    println!("\n📊 Cache Refresh Results:");
    println!("{}", result);

    // Check if caches are loaded
    let caches = pm_whale_follower::market_cache::global_caches();
    let neg_risk_count = caches.neg_risk.read().unwrap().len();
    let slug_count = caches.slugs.read().unwrap().len();
    let atp_count = caches.atp_tokens.read().unwrap().len();
    let ligue1_count = caches.ligue1_tokens.read().unwrap().len();
    let live_count = caches.live_status.read().unwrap().len();

    println!("\n📈 Cache Statistics:");
    println!("   Neg Risk: {} tokens", neg_risk_count);
    println!("   Slugs: {} tokens", slug_count);
    println!("   ATP Tokens: {} tokens", atp_count);
    println!("   Ligue1 Tokens: {} tokens", ligue1_count);
    println!("   Live Status: {} tokens", live_count);

    if neg_risk_count == 0 && slug_count == 0 {
        println!("\n⚠️  Warning: No data loaded. This may indicate:");
        println!("   - Network connectivity issues");
        println!("   - API rate limiting");
        println!("   - Cache files don't exist (this is normal on first run)");
    } else {
        println!("\n✅ Cache refresh completed successfully!");
    }

    println!();
    Ok(())
}

