# SportsX Smart Contracts

Solana smart contracts for the SportsX ecosystem, featuring a Proof of Fandom (PoF) points system and daily check-in rewards.

## Contracts

### 1. SportsX PoF (Proof of Fandom)
Core points management system.

**Features**:
- Wallet point tracking with dedicated PDA accounts
- Flexible point updates by admin or authorized contracts  
- Public point queries
- Contract authorization system (max 10 authorized contracts)

**Program ID**: `E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV`

### 2. SportsX Check-in
Daily check-in system that rewards users with PoF points via Cross-Program Invocation (CPI).

**Features**:
- 24-hour check-in interval enforcement
- Timestamp tracking for each wallet
- Automatic 10-point reward per check-in (via CPI to PoF contract)
- Check-in history and status queries

**Program ID**: `2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX`

### 3. Ticketing Program
Complete event ticketing system with marketplace, backend authorization, and PoF integration.

**Features**:
- **Platform Management**: Configuration, fee settings, pause/resume controls
- **Event Management**: Create events, manage status, configure check-in operators
- **Ticket Purchase**: Backend-authorized ticket purchase with USDC payment
- **NFT Marketplace**: List/buy/cancel ticket resales with fee collection
- **Check-in System**: On-site ticket verification with PoF rewards
- **Security**: Ed25519 signature verification for backend authorization

**Program ID**: `EFuMNTn1zfn6Zhvdq1Vjaxs83sz2gTWvDgjuJcKDYjhw`

## Architecture

Built with Anchor framework on Solana.

### PoF Contract

**Accounts**:
- `GlobalState`: Admin pubkey + authorized contracts list (max 10)
- `WalletPoints`: Per-wallet PDA storing point balance

**Instructions**:
- `initialize`: Setup global state with admin
- `initialize_wallet`: Create point account (starts at 0)
- `update_points`: Modify points (admin/authorized only)
- `get_points`: Query wallet points
- `authorize_contract` / `revoke_contract`: Manage authorized contracts

### Check-in Contract

**Accounts**:
- `CheckinRecord`: Per-wallet PDA tracking last check-in time and total count

**Instructions**:
- `initialize_checkin`: Create check-in record for wallet
- `daily_checkin`: Perform check-in (24hr cooldown) and receive 10 PoF points via CPI
- `get_checkin_info`: Query check-in status and next available time

### Ticketing Contract

**Accounts**:
- `PlatformConfig`: Platform settings (fees, authorities, pause status)
- `EventAccount`: Event details (times, metadata, resale rules)
- `TicketAccount`: Individual ticket NFTs with ownership and seat info
- `ListingAccount`: Marketplace listings for ticket resale
- `NonceTracker`: Replay attack prevention for backend signatures
- `TicketAuthority`: Authority PDA for PoF integration
- `CheckInAuthority`: Check-in operator management per event

**Instructions**:
- **Platform**: `initialize_platform`, `update_platform_config`, `toggle_pause`, `transfer_authority`
- **Events**: `create_event`, `update_event_status`, `add_checkin_operator`, `remove_checkin_operator`
- **Purchase**: `purchase_ticket` (backend-authorized with USDC payment)
- **Marketplace**: `list_ticket`, `buy_listed_ticket`, `cancel_listing`
- **Check-in**: `check_in_ticket` (rewards PoF points via CPI)

## Documentation

### Quick Start
- **FRONTEND_INTEGRATION.md** - Complete frontend integration guide with React examples

### Detailed Guides
For comprehensive guides, see the `.docs/` directory:
- **QUICKSTART.md** - Quick start guide (Chinese) with test steps
- **USAGE_GUIDE.md** - Comprehensive usage guide with examples
- **PROJECT_STRUCTURE.md** - Detailed project structure documentation

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)
- [Anchor](https://www.anchor-lang.com/docs/installation)
- [Node.js](https://nodejs.org/) (v16 or later)
- [Yarn](https://yarnpkg.com/)

## Installation

```bash
# Install dependencies
yarn install

# Build the program
anchor build

# Run tests
anchor test
```

## Deployment

### Local Deployment

```bash
# Start local validator
solana-test-validator

# Deploy to local
anchor deploy
```

### Devnet Deployment

```bash
# Set cluster to devnet
solana config set --url devnet

# Get some SOL for deployment
solana airdrop 2

# Deploy
anchor deploy --provider.cluster devnet
```

## Usage Examples

### Initialize Global State

```typescript
const [globalStatePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("global_state")],
  program.programId
);

await program.methods
  .initialize()
  .accounts({
    globalState: globalStatePda,
    admin: admin.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### Initialize Wallet Points

```typescript
const [walletPointsPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("wallet_points"), wallet.publicKey.toBuffer()],
  program.programId
);

await program.methods
  .initializeWallet()
  .accounts({
    walletPoints: walletPointsPda,
    wallet: wallet.publicKey,
    payer: payer.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .signers([payer])
  .rpc();
```

### Update Points (Admin)

```typescript
// Increase points
await program.methods
  .updatePoints(new anchor.BN(100))
  .accounts({
    walletPoints: walletPointsPda,
    globalState: globalStatePda,
    authority: admin.publicKey,
  })
  .rpc();

// Decrease points
await program.methods
  .updatePoints(new anchor.BN(-30))
  .accounts({
    walletPoints: walletPointsPda,
    globalState: globalStatePda,
    authority: admin.publicKey,
  })
  .rpc();
```

### Query Points

```typescript
const points = await program.methods
  .getPoints()
  .accounts({
    walletPoints: walletPointsPda,
  })
  .view();

console.log(`Wallet has ${points} points`);
```

### Authorize Contract

```typescript
await program.methods
  .authorizeContract(contractPublicKey)
  .accounts({
    globalState: globalStatePda,
    admin: admin.publicKey,
  })
  .rpc();
```

### Revoke Contract

```typescript
await program.methods
  .revokeContract(contractPublicKey)
  .accounts({
    globalState: globalStatePda,
    admin: admin.publicKey,
  })
  .rpc();
```

## Security Features

- **Authorization Checks**: Only admin or authorized contracts can update points
- **Overflow Protection**: Safe math operations prevent point overflow
- **Negative Balance Prevention**: Cannot reduce points below zero
- **PDA-based Accounts**: Deterministic addresses for global state and wallet points
- **Max Authorized Contracts**: Limited to 10 authorized contracts to prevent bloat

## Error Codes

- `Unauthorized`: Only admin or authorized contracts can perform this action
- `PointsOverflow`: Point calculation resulted in overflow
- `InsufficientPoints`: Cannot reduce points below zero
- `ContractAlreadyAuthorized`: Contract is already in authorized list
- `ContractNotAuthorized`: Contract is not in authorized list
- `MaxAuthorizedContractsReached`: Maximum of 10 authorized contracts reached

## Program Structure

```
sportsx-solana-contract/
├── Anchor.toml                  # Anchor configuration
├── Cargo.toml                  # Workspace configuration
├── package.json                # Node dependencies
├── programs/
│   ├── sportsx-pof/           # PoF points system
│   │   ├── Cargo.toml
│   │   ├── Xargo.toml
│   │   └── src/lib.rs
│   ├── sportsx-checkin/       # Daily check-in rewards
│   │   ├── Cargo.toml
│   │   ├── Xargo.toml
│   │   └── src/lib.rs
│   └── ticketing_program/     # Event ticketing & marketplace
│       ├── Cargo.toml
│       ├── Xargo.toml
│       └── src/
│           ├── lib.rs
│           ├── errors.rs
│           ├── instructions/  # Platform, event, purchase, marketplace, check-in
│           └── state/         # Account structures
├── tests/
│   ├── sportsx-pof.ts
│   ├── sportsx-checkin.ts
│   └── ticketing.ts
└── scripts/                   # Deployment & initialization scripts
```

## Testing

The test suite covers:

**PoF Contract**:
- Global state initialization
- Wallet points initialization
- Point updates (increase/decrease) by admin
- Point queries
- Unauthorized access prevention
- Contract authorization/revocation
- Authorized contract point updates
- Edge cases (negative balance, double authorization, etc.)

**Check-in Contract**:
- Check-in record initialization
- Daily check-in with 24-hour cooldown
- PoF points rewards via CPI
- Check-in status queries

**Ticketing Contract**:
- Platform initialization and configuration
- Event creation and management
- Ticket purchase with backend authorization
- Marketplace listing, buying, and cancellation
- Check-in flow with PoF rewards
- Security (signature verification, nonce replay prevention)

Run tests with:
```bash
anchor test
```

## License

MIT

## Contributing

Contributions are welcome! Please ensure all tests pass before submitting a PR.

## Contact

For questions or support, please open an issue in the repository.

