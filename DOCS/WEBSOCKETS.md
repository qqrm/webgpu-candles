# WebSocket Integration

This project uses the Binance WebSocket API to receive real‑time candlestick data. The `BinanceWebSocketClient` defined in `src/infrastructure/websocket/binance_client.rs` manages the connection.

## Stream Endpoint

A WebSocket connection is opened to:

```
wss://stream.binance.com:9443/ws/{symbol}@kline_{interval}
```

where `symbol` is the trading pair like `BTCUSDT` and `interval` is a value such as `1m`.

## Message Format

Incoming messages contain a `k` field with kline information:

```json
{
  "k": {
    "t": 123456789,
    "o": "10000.0",
    "h": "10100.0",
    "l": "9900.0",
    "c": "10050.0",
    "v": "10.0"
  }
}
```

The client parses this structure and converts it to the domain `Candle` type. Values are validated and then propagated to the UI layer for rendering.
