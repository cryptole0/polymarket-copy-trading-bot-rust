# Hyperliquid Market Making Bot

A sophisticated market making bot for Hyperliquid DEX that provides liquidity by continuously quoting buy and sell prices, earning profits from bid-ask spreads.

## Overview

Market making bots provide liquidity to decentralized exchanges by:
- Continuously placing buy and sell orders around the current market price
- Maintaining a spread between bid and ask prices
- Earning profits from the spread while facilitating trading
- Managing positions and risk automatically

This bot is designed for high-volume traders who want to automate liquidity provision on Hyperliquid.

## Features

- **Automated Market Making**: Continuously quotes buy/sell prices with configurable spread
- **Real-Time Order Book Monitoring**: WebSocket integration for live market data
- **Position Management**: Automatic position tracking and risk management
- **Configurable Parameters**: Adjustable spread, order size, position limits, and update intervals
- **Order Management**: Automatic order placement and cancellation
- **Error Handling**: Robust error handling with automatic reconnection

## Prerequisites

- Node.js 16+ and npm/yarn
- TypeScript 5.7+
- Hyperliquid account and API credentials
- Private key for signing transactions

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd hyperliquid-market-making-bot
```

2. Install dependencies:
```bash
npm install
# or
yarn install
```

3. Create a `.env` file in the root directory:
```env
# Hyperliquid API Configuration
HYPERLIQUID_API_URL=https://api.hyperliquid.xyz
HYPERLIQUID_WS_URL=wss://api.hyperliquid.xyz/ws

# Trading Configuration
PRIVATE_KEY=your_private_key_here
SYMBOL=ETH
BASE_URL=https://api.hyperliquid.xyz

# Market Making Parameters
SPREAD_PERCENTAGE=0.1
ORDER_SIZE=0.01
MAX_POSITION_SIZE=1.0
UPDATE_INTERVAL_MS=1000
PRICE_TICK_SIZE=0.01
```

## Configuration

### Environment Variables

- `HYPERLIQUID_API_URL`: Hyperliquid REST API endpoint
- `HYPERLIQUID_WS_URL`: Hyperliquid WebSocket endpoint
- `PRIVATE_KEY`: Your private key for signing transactions
- `SYMBOL`: Trading pair symbol (e.g., ETH, BTC)
- `SPREAD_PERCENTAGE`: Spread percentage (e.g., 0.1 for 0.1%)
- `ORDER_SIZE`: Size of each order in base currency
- `MAX_POSITION_SIZE`: Maximum position size allowed
- `UPDATE_INTERVAL_MS`: How often to update quotes (milliseconds)
- `PRICE_TICK_SIZE`: Minimum price movement

## Usage

### Development Mode
```bash
npm run dev
```

### Production Mode
```bash
npm run build
npm start
```

## How It Works

1. **Initialization**: The bot connects to Hyperliquid API and WebSocket, fetches current order book and position
2. **Market Making Loop**: 
   - Calculates mid price from current order book
   - Places bid order below mid price and ask order above mid price
   - Maintains configured spread percentage
   - Updates orders periodically based on market conditions
3. **Position Management**: Monitors position size and prevents exceeding limits
4. **Order Management**: Automatically cancels and replaces orders to maintain quotes

## Risk Management

- Maximum position size limits prevent excessive exposure
- Automatic order cancellation on shutdown
- Position monitoring and warnings
- Configurable risk parameters

## Important Notes

⚠️ **Disclaimer**: Trading cryptocurrencies involves substantial risk. This bot is provided as-is for educational purposes. Always:
- Test thoroughly on testnet before using real funds
- Start with small position sizes
- Monitor the bot closely
- Understand the risks involved
- The creators are not responsible for any financial losses

## API Integration

This bot uses the Hyperliquid API. You may need to:
1. Sign up for Hyperliquid API access
2. Generate API keys
3. Implement proper authentication/signing based on Hyperliquid's requirements
4. Adjust API endpoints and request formats based on the latest Hyperliquid documentation

**Note**: The current implementation includes placeholder API calls. You'll need to implement the actual Hyperliquid API integration based on their official documentation.

## Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## License

ISC

## Support

For issues, questions, or contributions, please open an issue on the repository.
