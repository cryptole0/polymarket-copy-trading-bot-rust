// Order types
export type OrderSide = 'buy' | 'sell';
export type OrderType = 'limit' | 'market';

export interface Order {
  symbol: string;
  side: OrderSide;
  type: OrderType;
  size: number;
  price?: number;
  orderId?: string;
  timestamp?: number;
}

export interface OrderBookLevel {
  price: number;
  size: number;
}

export interface OrderBook {
  bids: OrderBookLevel[];
  asks: OrderBookLevel[];
  timestamp: number;
}

export interface Position {
  symbol: string;
  size: number;
  entryPrice: number;
  unrealizedPnl: number;
}

export interface MarketData {
  symbol: string;
  bid: number;
  ask: number;
  last: number;
  volume24h: number;
  timestamp: number;
}

export interface MarketMakingConfig {
  symbol: string;
  spreadPercentage: number;
  orderSize: number;
  maxPositionSize: number;
  updateIntervalMs: number;
  priceTickSize: number;
}