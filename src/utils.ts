import chalk from 'chalk';

// Logging utilities
const log = {
  info: (message: string) => console.log(chalk.blue(`[INFO] ${message}`)),
  success: (message: string) => console.log(chalk.green(`[SUCCESS] ${message}`)),
  error: (message: string) => console.error(chalk.red(`[ERROR] ${message}`)),
  warn: (message: string) => console.warn(chalk.yellow(`[WARN] ${message}`)),
  debug: (message: string) => console.log(chalk.gray(`[DEBUG] ${message}`)),
};

// Utility functions
const sleep = (ms: number): Promise<void> => {
  return new Promise(resolve => setTimeout(resolve, ms));
};

const roundToTick = (price: number, tickSize: number): number => {
  return Math.round(price / tickSize) * tickSize;
};

const formatPrice = (price: number, decimals: number = 2): string => {
  return price.toFixed(decimals);
};

const formatSize = (size: number, decimals: number = 4): string => {
  return size.toFixed(decimals);
};

export {
  log,
  sleep,
  roundToTick,
  formatPrice,
  formatSize
}