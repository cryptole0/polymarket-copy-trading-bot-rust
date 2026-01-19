import axios from 'axios';
import WebSocket from 'ws';
import {
  BASE_URL,
  SYMBOL,
  SPREAD_PERCENTAGE,
  ORDER_SIZE,
  MAX_POSITION_SIZE,
  UPDATE_INTERVAL_MS,
  PRICE_TICK_SIZE
} from './config';
import { log, sleep, roundToTick, formatPrice, formatSize } from './utils';
import { Order, OrderBook, MarketData, Position, MarketMakingConfig } from './types';

export class MarketMakingBot {
  private config: MarketMakingConfig;
  private orderBook: OrderBook | null = null;
  private currentPosition: Position | null = null;
  private activeOrders: Map<string, Order> = new Map();
  private ws: WebSocket | null = null;
  private isRunning: boolean = false;

  constructor() {
    this.config = {
      symbol: SYMBOL,
      spreadPercentage: SPREAD_PERCENTAGE,
      orderSize: ORDER_SIZE,
      maxPositionSize: MAX_POSITION_SIZE,
      updateIntervalMs: UPDATE_INTERVAL_MS,
      priceTickSize: PRICE_TICK_SIZE
    };
  }

  async start(): Promise<void> {
    log.info(`Starting market making bot for ${this.config.symbol}`);
    this.isRunning = true;

    // Initialize connection and fetch initial data
    await this.initialize();
    
    // Start market making loop
    await this.runMarketMakingLoop();
  }

  private async initialize(): Promise<void> {
    try {
      // Fetch initial order book
      await this.fetchOrderBook();
      
      // Fetch current position
      await this.fetchPosition();
      
      // Connect to WebSocket for real-time updates
      await this.connectWebSocket();
      
      log.success('Initialization complete');
    } catch (error: any) {
      log.error(`Initialization failed: ${error.message}`);
      throw error;
    }
  }

  private async fetchOrderBook(): Promise<void> {
    try {
      const response = await axios.get(`${BASE_URL}/l2Book`, {
        params: { coin: this.config.symbol }
      });
      
      // Parse order book data (adjust based on actual Hyperliquid API response)
      const data = response.data;
      this.orderBook = {
        bids: data.bids?.map(([price, size]: [string, string]) => ({
          price: parseFloat(price),
          size: parseFloat(size)
        })) || [],
        asks: data.asks?.map(([price, size]: [string, string]) => ({
          price: parseFloat(price),
          size: parseFloat(size)
        })) || [],
        timestamp: Date.now()
      };
      
      log.info(`Order book updated: ${this.orderBook.bids.length} bids, ${this.orderBook.asks.length} asks`);
    } catch (error: any) {
      log.error(`Failed to fetch order book: ${error.message}`);
      throw error;
    }
  }

  private async fetchPosition(): Promise<void> {
    try {
      // Fetch current position from Hyperliquid API
      // This is a placeholder - adjust based on actual API
      const response = await axios.get(`${BASE_URL}/info`, {
        params: { user: 'your_address' } // Replace with actual user address
      });
      
      // Parse position data (adjust based on actual API response)
      const positionData = response.data; // Adjust based on actual response structure
      
      if (positionData && positionData.size !== 0) {
        this.currentPosition = {
          symbol: this.config.symbol,
          size: parseFloat(positionData.size || '0'),
          entryPrice: parseFloat(positionData.entryPrice || '0'),
          unrealizedPnl: parseFloat(positionData.unrealizedPnl || '0')
        };
        log.info(`Current position: ${formatSize(this.currentPosition.size)} @ ${formatPrice(this.currentPosition.entryPrice)}`);
      } else {
        this.currentPosition = null;
        log.info('No open position');
      }
    } catch (error: any) {
      log.warn(`Failed to fetch position: ${error.message}`);
      // Continue even if position fetch fails
    }
  }

  private async connectWebSocket(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        // Connect to Hyperliquid WebSocket (adjust URL and params as needed)
        const wsUrl = `wss://api.hyperliquid.xyz/ws`;
        this.ws = new WebSocket(wsUrl);
        
        this.ws.on('open', () => {
          log.success('WebSocket connected');
          // Subscribe to order book updates
          this.ws?.send(JSON.stringify({
            method: 'subscribe',
            subscription: { type: 'l2Book', coin: this.config.symbol }
          }));
          resolve();
        });
        
        this.ws.on('message', (data: WebSocket.Data) => {
          try {
            const message = JSON.parse(data.toString());
            this.handleWebSocketMessage(message);
          } catch (error: any) {
            log.error(`Failed to parse WebSocket message: ${error.message}`);
          }
        });
        
        this.ws.on('error', (error: Error) => {
          log.error(`WebSocket error: ${error.message}`);
        });
        
        this.ws.on('close', () => {
          log.warn('WebSocket closed');
          if (this.isRunning) {
            // Attempt to reconnect
            setTimeout(() => this.connectWebSocket(), 5000);
          }
        });
      } catch (error: any) {
        reject(error);
      }
    });
  }

  private handleWebSocketMessage(message: any): void {
    // Handle WebSocket updates (order book, trades, etc.)
    if (message.type === 'l2Book') {
      this.updateOrderBookFromWS(message.data);
    }
  }

  private updateOrderBookFromWS(data: any): void {
    // Update order book from WebSocket data
    if (this.orderBook) {
      this.orderBook.bids = data.bids?.map(([price, size]: [string, string]) => ({
        price: parseFloat(price),
        size: parseFloat(size)
      })) || this.orderBook.bids;
      
      this.orderBook.asks = data.asks?.map(([price, size]: [string, string]) => ({
        price: parseFloat(price),
        size: parseFloat(size)
      })) || this.orderBook.asks;
      
      this.orderBook.timestamp = Date.now();
    }
  }

  private async runMarketMakingLoop(): Promise<void> {
    while (this.isRunning) {
      try {
        // Refresh order book periodically
        await this.fetchOrderBook();
        
        // Calculate and place quotes
        await this.updateQuotes();
        
        // Check and manage position
        await this.managePosition();
        
        await sleep(this.config.updateIntervalMs);
      } catch (error: any) {
        log.error(`Error in market making loop: ${error.message}`);
        await sleep(this.config.updateIntervalMs);
      }
    }
  }

  private async updateQuotes(): Promise<void> {
    if (!this.orderBook || this.orderBook.bids.length === 0 || this.orderBook.asks.length === 0) {
      log.warn('Order book not available, skipping quote update');
      return;
    }

    const midPrice = (this.orderBook.bids[0].price + this.orderBook.asks[0].price) / 2;
    const spread = midPrice * (this.config.spreadPercentage / 100);
    
    const bidPrice = roundToTick(midPrice - spread / 2, this.config.priceTickSize);
    const askPrice = roundToTick(midPrice + spread / 2, this.config.priceTickSize);

    log.info(`Mid price: ${formatPrice(midPrice)}, Bid: ${formatPrice(bidPrice)}, Ask: ${formatPrice(askPrice)}`);

    // Cancel existing orders
    await this.cancelAllOrders();

    // Place new orders
    await this.placeOrder('buy', bidPrice, this.config.orderSize);
    await this.placeOrder('sell', askPrice, this.config.orderSize);
  }

  private async placeOrder(side: OrderSide, price: number, size: number): Promise<void> {
    try {
      // Check position limits
      if (side === 'buy' && this.currentPosition) {
        const newPositionSize = this.currentPosition.size + size;
        if (Math.abs(newPositionSize) > this.config.maxPositionSize) {
          log.warn(`Would exceed max position size, skipping ${side} order`);
          return;
        }
      }

      if (side === 'sell' && this.currentPosition) {
        const newPositionSize = this.currentPosition.size - size;
        if (Math.abs(newPositionSize) > this.config.maxPositionSize) {
          log.warn(`Would exceed max position size, skipping ${side} order`);
          return;
        }
      }

      // Place order via Hyperliquid API
      const order: Order = {
        symbol: this.config.symbol,
        side,
        type: 'limit',
        size,
        price
      };

      // This is a placeholder - implement actual API call based on Hyperliquid documentation
      log.info(`Placing ${side} order: ${formatSize(size)} @ ${formatPrice(price)}`);
      
      // Example API call (adjust based on actual Hyperliquid API)
      // const response = await axios.post(`${BASE_URL}/exchange`, {
      //   action: { type: 'order', orders: [order] },
      //   nonce: Date.now(),
      //   signature: this.signOrder(order)
      // });
      
      // Store active order
      const orderId = `order_${Date.now()}_${side}`;
      order.orderId = orderId;
      order.timestamp = Date.now();
      this.activeOrders.set(orderId, order);
      
      log.success(`Order placed: ${orderId}`);
    } catch (error: any) {
      log.error(`Failed to place ${side} order: ${error.message}`);
    }
  }

  private async cancelAllOrders(): Promise<void> {
    for (const [orderId, order] of this.activeOrders) {
      try {
        // Cancel order via API
        log.info(`Cancelling order: ${orderId}`);
        
        // Example API call (adjust based on actual Hyperliquid API)
        // await axios.post(`${BASE_URL}/exchange`, {
        //   action: { type: 'cancel', orders: [{ orderId }] },
        //   nonce: Date.now(),
        //   signature: this.signCancel(orderId)
        // });
        
        this.activeOrders.delete(orderId);
      } catch (error: any) {
        log.error(`Failed to cancel order ${orderId}: ${error.message}`);
      }
    }
  }

  private async managePosition(): Promise<void> {
    if (!this.currentPosition) {
      return;
    }

    const positionSize = Math.abs(this.currentPosition.size);
    
    if (positionSize > this.config.maxPositionSize * 0.8) {
      log.warn(`Position size (${formatSize(positionSize)}) approaching limit`);
      // Implement position reduction logic if needed
    }
  }

  async stop(): Promise<void> {
    log.info('Stopping market making bot...');
    this.isRunning = false;
    
    // Cancel all active orders
    await this.cancelAllOrders();
    
    // Close WebSocket connection
    if (this.ws) {
      this.ws.close();
    }
    
    log.success('Bot stopped');
  }
}
