# CW20 Mint and Burn

This smart contract mints CW20 tokens when users send `uluna` to the contract, burns the `uluna`, and transfers the minted tokens back to the sender.

## Setup
- `cargo wasm`
- Use `cargo schema` to update JSON schemas for the messages.

## Usage
- Admin can set CW20 token address.
- Users send `uluna` to mint equivalent CW20 tokens.