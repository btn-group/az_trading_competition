# Aleph Zero Trading Competition Smart Contract

### Notes about allowed bools

Only pools with large market caps will be allowed.

AZERO/USDC: 5C6s2qJAG5dCmPvR9WyKAVL6vJRDS9BjMwbrqwXGCsPiFViF
USDC/USDT: 5CiP96MhEGHnLFGS64uVznrwbuVdFj6kewrEZoLRzxUEqxws
WAZERO/WETH: 5HaM6dHg3ymuQ6NSCquMkzBLLHv9t1H4YvBDMarox37PbusE

### Note about DIA oracle for decentralising winner selection

It's not possible to use the DIA oracle right now as you can't specify a time. In an ideal situation, we would want the first price after the end timestamp but the oracle only gives you the latest timestamp. Unless a function to get the prices was called immediately after the end, people would be unhappy about the result. Even if someone was ready to call, if a network issue happened with either DIA or Aleph Zero, this could cause massive problems.

For the meanwhile, the best option is to select official sources for price that have an api for prices with time range.

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
