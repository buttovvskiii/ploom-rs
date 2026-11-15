import type { RawExchange } from "$lib/api";

export interface LoggedExchange extends RawExchange {
  id: number;
  at: number;
}

class ExchangeLog {
  items = $state<LoggedExchange[]>([]);
  private nextId = 1;

  add(exchange: RawExchange): void {
    this.items = [{ ...exchange, id: this.nextId++, at: Date.now() }, ...this.items].slice(0, 50);
  }

  clear(): void {
    this.items = [];
  }
}

export const exchanges = new ExchangeLog();
