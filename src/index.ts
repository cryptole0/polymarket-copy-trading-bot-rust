import { config } from 'dotenv';
import { MarketMakingBot } from './bot';
import { log } from './utils';
import { ABOUT_BOT_STR } from './constants';

config();

async function main() {
  log.info('Starting Hyperliquid Market Making Bot...');
  console.log(ABOUT_BOT_STR);
  
  try {
    const bot = new MarketMakingBot();
    await bot.start();
  } catch (error: any) {
    log.error(`Failed to start bot: ${error.message}`);
    process.exit(1);
  }
}

// Handle graceful shutdown
process.on('SIGINT', () => {
  log.warn('Received SIGINT, shutting down gracefully...');
  process.exit(0);
});

process.on('SIGTERM', () => {
  log.warn('Received SIGTERM, shutting down gracefully...');
  process.exit(0);
});

main();