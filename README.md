# Sealed-Bid Auction Factory (Commit-Reveal)

A smart contract **factory** that deploys commit-reveal style sealed-bid auction instances for ERC-721 NFTs on **Arbitrum Stylus (Rust)**. This design mirrors the Dutch Auction Factory pattern, but the mechanism is different: auctions are conducted in a **sealed-bid, two-phase process** (commit → reveal).

---

## 📌 Features

* **Commit-Reveal Bidding**: Bidders first commit to their bid hash, then reveal later with bid + nonce.
* **Factory Deployment**: Uses `RawDeploy` and `CREATE2` to deterministically deploy new auction instances.
* **Embedded Bytecode**: The compiled `sealed_bid_auction.wasm` is embedded into the factory at compile time.
* **Auction Registry**: Each auction has a unique ID and is mapped to its deployed address.
* **Admin Controls**: Factory owner can pause or unpause auction creation.
* **Gas Efficient**: Deployment uses pre-compiled WASM with no runtime constructor args.

---

## ⚡ Auction Flow (High-Level)

1. **Auction Creation**: Factory deploys a new `sealed_bid_auction` instance with parameters:

   * NFT contract + token\_id
   * Reserve price
   * Commit phase duration
   * Reveal phase duration
   * Minimum deposit

2. **Commit Phase**: Bidders lock in their bids using `keccak256(bid || nonce)` and a deposit.

3. **Reveal Phase**: Bidders reveal their bid + nonce. Invalid, unrevealed, or too-low bids are discarded.

4. **Settlement**: The highest valid revealed bid ≥ reserve wins. NFT is transferred to winner, funds to seller, refunds to losers.

---

## 🔧 Prerequisites

* [Rust](https://rustup.rs/) toolchain
* [Cargo Stylus](https://github.com/OffchainLabs/cargo-stylus)
* A compiled `sealed_bid_auction.wasm` contract (the auction instance)

---

## 🚀 Build Process

### 1. Build Auction Instance

```bash
cd ../sealed_bid_auction
cargo stylus check
```

### 2. Build Factory

```bash
cd ../sealed_bid_auction_factory
rustup target add wasm32-unknown-unknown
cargo install cargo-stylus
cargo stylus check
```

⚠️ The factory embeds:

```
../sealed_bid_auction/target/wasm32-unknown-unknown/release/sealed_bid_auction.wasm
```

---

## 📦 Deployment

Deploy to **Arbitrum Sepolia** (testnet):

```bash
cargo stylus deploy \
  --endpoint <RPC_URL> \
  --private-key <YOUR_PRIVATE_KEY>
```

---

## 🛠️ Usage

### Create Auction

```rust
create_auction(
  nft_contract: Address,
  token_id: U256,
  reserve_price: U256,
  commit_duration: U256,
  reveal_duration: U256,
  min_deposit: U256
) -> Result<Address, Vec<u8>>
```

Returns the deployed auction instance address.

### View Functions

```rust
get_auction(id: U256) -> Address
get_creator(id: U256) -> Address
get_auction_count() -> U256
get_owner() -> Address
is_paused() -> bool
get_bytecode_length() -> U256
```

### Admin Functions

```rust
pause() -> Result<(), Vec<u8>>
unpause() -> Result<(), Vec<u8>>
```

---

## 🏗️ Instance Contract (Sealed-Bid Auction)

The factory only deploys auctions; logic lives in the instance (`sealed_bid_auction`). A typical instance would have:

* **Storage**:

  * `seller`, `nft_contract`, `token_id`
  * `reserve_price`, `min_deposit`
  * `commit_end`, `reveal_end`
  * `commitments: mapping(address => bytes32)`
  * `revealed_bids: mapping(address => U256)`
  * `highest_bid`, `highest_bidder`
  * `settled: bool`

* **Functions**:

  * `commit(commitment) payable`
  * `reveal(bid, nonce) payable`
  * `finalize()`
  * `refund()`
  * `cancel()` (if no bids)

---

## 🧭 Architecture

```
┌────────────────────────────┐
│ SealedBidAuctionFactory    │
│                            │
│ ┌────────────────────────┐ │
│ │ Embedded WASM Bytecode │ │
│ │ (sealed_bid_auction)   │ │
│ └────────────────────────┘ │
│                            │
│ create_auction()           │
│   ↓                        │
│ RawDeploy + CREATE2        │
└───────────────┬────────────┘
                ↓
┌──────────────────────────────────┐
│ New SealedBidAuction Instance    │
│ (commit → reveal → finalize)    │
└──────────────────────────────────┘
```

---

## 🔒 Security

* **Input Validation**: Non-zero durations, deposits, and valid addresses checked.
* **Deterministic Deployment**: CREATE2 prevents address collisions.
* **Owner Control**: Factory owner can pause/unpause.
* **Commit-Reveal Safety**: Prevents bid sniping and promotes fairness.

---

## 📜 License

Licensed under MIT OR Apache-2.0.
