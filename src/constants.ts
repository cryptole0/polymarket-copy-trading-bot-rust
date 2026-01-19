import { SYMBOL, SPREAD_PERCENTAGE, ORDER_SIZE } from "./config";

const ABOUT_BOT_STR = `Hyperliquid Market Making Bot v1.0

This bot provides liquidity on Hyperliquid DEX by continuously quoting buy and sell prices.
It earns profits from the bid-ask spread while maintaining market depth.

Features:
- Automated market making with configurable spread
- Real-time order book monitoring
- Position and risk management
- Continuous quote updates

Symbol: ${SYMBOL}
Spread: ${SPREAD_PERCENTAGE}%
Order Size: ${ORDER_SIZE}
`;

const API_ENDPOINTS = {
  EXCHANGE_INFO: "/exchange",
  USER_STATE: "/info",
  ORDERBOOK: "/l2Book",
  TRADES: "/trades",
  PLACE_ORDER: "/exchange",
  CANCEL_ORDER: "/exchange",
};

export {
  ABOUT_BOT_STR,
  API_ENDPOINTS
}