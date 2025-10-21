# SportsX Ticketing Contract

A decentralized ticketing system built on Solana using the Anchor framework.

## Overview

This smart contract provides a complete Web3 ticketing solution with:

- ✅ **Platform Management**: Configurable fees, pause mechanism, multisig authority
- 🎫 **Event Management**: Flexible event creation with custom ticket types
- 🔐 **Secure Purchases**: Backend-authorized transactions with anti-replay protection
- 💱 **Secondary Market**: Built-in resale with configurable fees
- ✓ **Check-in System**: Operator-based ticket validation

## Project Structure

```
├── programs/ticketing_program/
│   └── src/
│       ├── lib.rs                    # Program entry point
│       ├── state/                    # Account structures (7 types)
│       ├── instructions/             # Instruction handlers (modular)
│       └── errors.rs                 # Error definitions
├── tests/                            # Integration tests
├── migrations/                       # Deployment scripts
└── .docs/                           # Documentation
```

## Quick Start

```bash
# Install dependencies
yarn install

# Build
anchor build

# Run tests (automated)
yarn test:auto

# Or run tests directly
anchor test

# Deploy
anchor deploy
```

## Running Tests

### Automated Test Suite
```bash
# Full test suite with reporting
./scripts/run-tests.sh

# Or via npm script
yarn test:auto
```

### Test Coverage
```bash
# View test coverage analysis
./scripts/test-coverage.sh

# Or via npm script
yarn test:coverage
```

### Available Commands
- `yarn test` - Run Anchor tests
- `yarn test:auto` - Automated test runner with reporting
- `yarn test:coverage` - Test coverage analysis
- `yarn build` - Build the program
- `yarn deploy:local` - Deploy to localnet
- `yarn deploy:devnet` - Deploy to devnet

## Documentation

- [Architecture](/.docs/ARCHITECTURE.md) - Detailed technical design
- [Quick Start](/.docs/QUICKSTART.md) - Development guide
- [README](/.docs/README.md) - Feature overview

## Key Features

### Modular Design
All code is organized into logical modules for maintainability:
- `state/` - 7 account types
- `instructions/` - 6 instruction modules
- Clean separation of concerns

### Security
- Backend signature verification
- Nonce-based anti-replay protection
- Role-based access control
- Emergency pause mechanism

### Economic Model
- Platform fee: 0.1 USDC (configurable)
- Organizer resale fee (basis points)
- Resale limit enforcement

## Program Instructions

**Platform**: `initialize_platform`, `update_platform_config`, `toggle_pause`, `transfer_authority`

**Events**: `create_event`, `update_event_status`, `add_checkin_operator`, `remove_checkin_operator`

**Tickets**: `create_ticket_type`, `batch_mint_tickets`

**Purchase**: `purchase_ticket`

**Marketplace**: `list_ticket`, `buy_listed_ticket`, `cancel_listing`

**Check-in**: `check_in_ticket`

## License

MIT

## Contributing

Contributions welcome! Please follow Solana and Anchor best practices.

