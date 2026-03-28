# Token Vault (Solana / Anchor)

A Solana program for token-gated subscriptions using SPL Token-2022. Users pay a token fee into a PDA-controlled vault to create and renew time-based access (`expires_at`). An admin controls pricing through a config account, and validity checks emit on-chain events for clients to consume. The project is built with Anchor and validated with TypeScript + Mocha/Chai tests.

## Architecture

```text
Admin Wallet -- set_price() -----------------> Config PDA (admin, price, duration, is_paused)
                                              ^
                                              | has_one = admin

User Wallet -- subscribe_to_vault() --------> Transfer CPI (Token-2022) --> Vault ATA (PDA authority)
                           |
                           +---------------> Subscription PDA (owner, expires_at)

User Wallet -- renew_subscription() --------> Transfer CPI + expiry extension

PDA Vault Authority -- mint_to_user() ------> Mint CPI (with_signer) to user ATA
```

| Instruction                     | What it does                                                               | Access                                       |
| ------------------------------- | -------------------------------------------------------------------------- | -------------------------------------------- |
| `initialize_token_subscription` | Initializes config, mint, and vault ATA                                    | First caller (becomes admin)                 |
| `mint_to_user`                  | Faucet-style mint to caller ATA via PDA mint authority                     | Anyone                                       |
| `set_price`                     | Updates subscription price in config                                       | Admin only (`has_one = admin`)               |
| `subscribe_to_vault`            | Transfers price to vault and creates/updates subscription expiry           | Any signer with enough tokens                |
| `renew_subscription`            | Transfers price again and extends expiry                                   | Any signer with subscription + enough tokens |
| `is_user_subcribed`             | Checks `expires_at` against clock and emits validity event                 | Anyone                                       |
| `deposit`                       | Transfers user tokens to pooled vault and updates per-user bookkeeping PDA | Any signer                                   |
| `withdraw`                      | Transfers from vault to user ATA using PDA signer, bounded by bookkeeping  | Bookkept user amount only                    |

## Key Technical Decisions

- PDA-based mint/vault authority (`authority` seed) ensures CPI mint/transfer signing is program-controlled, not wallet-controlled.
- `token_interface` is used for Token-2022 compatibility while keeping CPI calls generic (`transfer_checked`, `mint_to`).
- Pooled vault tokens are paired with per-user bookkeeping (`deposit_info` PDA) so withdrawals are constrained by tracked user quantity.

## How to Run

### Prerequisites

- Solana CLI (with local validator available)
- Anchor CLI
- Node.js + Yarn

### Commands

```bash
anchor build
anchor test
```

## Tech Stack

Rust, Anchor, TypeScript, SPL Token-2022, Mocha/Chai
