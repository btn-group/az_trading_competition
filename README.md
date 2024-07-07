# Aleph Zero Trading Competition Smart Contract

### Notes about allowed pools and tokens

- Only pools with large market caps will be allowed.
- Each token in pools must have a DIA price symbol associated with it.

AZERO/USDC: 5C6s2qJAG5dCmPvR9WyKAVL6vJRDS9BjMwbrqwXGCsPiFViF
USDC/USDT: 5CiP96MhEGHnLFGS64uVznrwbuVdFj6kewrEZoLRzxUEqxws
AZERO/ETH: 5HaM6dHg3ymuQ6NSCquMkzBLLHv9t1H4YvBDMarox37PbusE

### Note about DIA oracle for decentralising winner selection

- Free and available.
- Can't specify a time so it may have to come down to any user being able to call the record final price function.
- In case there's a problem with the oracle, will need a manual option that can be set after a certain amount of time.
- Will need to have a think about the price of the token in terms of charting and front end.

### Note about minting an NFT on registration

Was thinking the easiest way to go about it would be to:
1. create a collection on artzero
2. Whitelist the trading competition smart contract
3. Have the whitelist phase up until registration for trading competition ends
4. Setup the trading competition smart contract to mint a nft everytime someone registers and then send it to the user.
5. Rest of the collection will be made public after that.

During whitelist phase, the price would be 0, then public whatever you reckon.

### Notes about Subsquid and multiple tournaments

To show a leaderboard:
- Need all the competitors and a tracking of their token balances.
- Need the price from the DIA oracle.

So that we don't have to start or modify a squid every time a tournament starts, it'd be best to have one contract instance that can control multiple tournaments.

## Getting Started

### Prerequisites

* [Cargo](https://doc.rust-lang.org/cargo/)
* [Rust](https://www.rust-lang.org/)
* [ink!](https://use.ink/)
* [Cargo Contract v3.2.0](https://github.com/paritytech/cargo-contract)
```zsh
cargo install --force --locked cargo-contract --version 3.2.0
```

### Checking code

```zsh
cargo checkmate
cargo sort
```

## Testing

### Run unit tests

```sh
cargo test
```

## Deployment

1. Build contract:
```sh
# You may need to run
# chmod +x build.sh f
./build.sh
```
2. If setting up locally, start a local development chain.
```sh
substrate-contracts-node --dev
```
3. Upload, initialise and interact with contract at [Contracts UI](https://contracts-ui.substrate.io/).

## References

- [Ink env block timestamp](https://docs.rs/ink_env/4.0.0/ink_env/fn.block_timestamp.html)
- https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date/getMilliseconds
- [DIA Oracles on Aleph Zero](https://github.com/diadata-org/dia-oracle-anchor)
- https://github.com/ArtZero-io/Contracts/tree/mainnet/Azero_Contracts/contracts
- https://learn.brushfam.io/docs/OpenBrush/smart-contracts/PSP34/Extensions/metadata
