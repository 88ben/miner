# Miner

Multi-cryptocurrency mining engine with a web-based dashboard.

## Supported Coins

| Coin               | Symbol | Algorithm   | Hardware |
|--------------------|--------|-------------|----------|
| Monero             | XMR    | RandomX     | CPU      |
| Ethereum Classic   | ETC    | ETCHash     | GPU      |
| Ravencoin          | RVN    | KawPow      | GPU      |
| Kaspa              | KAS    | kHeavyHash  | GPU      |
| Zcash              | ZEC    | Equihash    | GPU      |

## Architecture

```
miner/
├── crates/
│   ├── core/        # Algorithm traits, types, config, stats
│   ├── stratum/     # Stratum protocol client (pool communication)
│   ├── worker/      # Mining engine & worker management
│   └── api/         # CLI entrypoint + Axum REST API
├── ui/              # React/TypeScript dashboard (Vite + Tailwind)
└── config.json      # Runtime configuration
```

## Prerequisites

- [Rust](https://rustup.rs/) (1.75+)
- [Node.js](https://nodejs.org/) (20+)

## Quick Start

### 1. Generate a config file

```bash
cargo run -- --init-config
```

Edit `config.json` with your wallet addresses and pool settings.

### 2. Start the miner

```bash
cargo run -- -c config.json
```

The API server starts on `http://127.0.0.1:3030` by default.

### 3. Start the dashboard (development)

```bash
cd ui
npm install
npm run dev
```

Open `http://localhost:5173` to view the dashboard.

## API Endpoints

| Method | Path                    | Description              |
|--------|-------------------------|--------------------------|
| GET    | `/api/stats`            | Current mining stats     |
| GET    | `/api/coins`            | Supported coins list     |
| POST   | `/api/workers/stop-all` | Stop all active workers  |

## Configuration

Example `config.json`:

```json
{
  "worker_name": "my-rig",
  "coins": [
    {
      "symbol": "XMR",
      "algorithm": "RandomX",
      "wallet": "YOUR_XMR_WALLET_ADDRESS",
      "pool": {
        "url": "pool.supportxmr.com",
        "port": 3333,
        "tls": false
      },
      "threads": 4,
      "enabled": true
    }
  ],
  "api": {
    "enabled": true,
    "host": "127.0.0.1",
    "port": 3030
  },
  "log_level": "info"
}
```

## License

MIT
