# SportsX Proof of Fandom (PoF) Smart Contract

A Solana smart contract for tracking and managing user engagement points (Proof of Fandom) in the SportsX ecosystem.

## Features

1. **Wallet Point Tracking**: Each wallet has a dedicated point account to track their engagement score
2. **Flexible Point Updates**: Admin and authorized contracts can increase or decrease wallet points
3. **Point Queries**: Anyone can query the current points for any wallet
4. **Contract Authorization**: Admin can authorize/revoke other smart contracts to update points

## Architecture

The contract uses Anchor framework and implements the following core functionalities:

### Accounts

- **GlobalState**: Stores admin pubkey and list of authorized contracts (max 10)
- **WalletPoints**: PDA for each wallet storing their point balance

### Instructions

1. `initialize`: Initialize the global state with admin authority
2. `initialize_wallet`: Create a point account for a wallet (starts at 0)
3. `update_points`: Update wallet points (admin or authorized contract only)
4. `get_points`: Query current points for a wallet
5. `authorize_contract`: Admin authorizes a contract to update points
6. `revoke_contract`: Admin revokes a contract's authorization

## Documentation

For detailed guides and examples, see the `.docs/` directory:
- **QUICKSTART.md** - Quick start guide (Chinese)
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
sportsx-pof-contract/
├── Anchor.toml              # Anchor configuration
├── Cargo.toml              # Workspace configuration
├── package.json            # Node dependencies
├── programs/
│   └── sportsx-pof/
│       ├── Cargo.toml      # Program dependencies
│       ├── Xargo.toml      # Cross-compilation config
│       └── src/
│           └── lib.rs      # Main program code
└── tests/
    └── sportsx-pof.ts     # Integration tests
```

## Testing

The test suite covers:

- Global state initialization
- Wallet points initialization
- Point updates (increase/decrease) by admin
- Point queries
- Unauthorized access prevention
- Contract authorization/revocation
- Authorized contract point updates
- Edge cases (negative balance, double authorization, etc.)

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

