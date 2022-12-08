# Zipper: An Anti-Rug & Anti-Sandwich Primitive

Transaction simulations can be spoofed; it is possible to have an accurate simulation of the execution of a transaction with a desired outcome and then observe a different outcome when executing the transaction in real time. 

On Solana, transactions are comprised of multiple instructions. If any instruction in the transaction fails, the entire transaction fails. Zipper takes advantage of this.

Zipper is an on-chain program that contains multiple versions of a single instruction. The instruction expects a user's pubkey, a set of token accounts (can be empty), and a set of expected balances. If the balances in the SOL account or token accounts are not **at least** those provided in the expected balances, the program panics and the transaction fails. 

This instruction can be included after any instruction that mutates a SOL or token account to ensure that the instruction does not take more lamports than what you expect. Note that only accounts that are included in a transaction and are marked as mutable need to be included. Since these accounts are already included in your transaction, the Zipper Program is the only additional account needed. Furthermore, an ordered `u64` array is used for the expected balances, which adds only 8 bytes per account to be checked.

## Example Usage
```rust
use anchor_client::{
    solana_sdk::instruction::Instruction,
    Client, Cluster, Program
};
use zipper::{AccountZipper, ID as ZIPPER_PROGRAM_ID};

// A sketchy ix that needs to mutable access to five
// token accounts, for whatever reason
let sketchy_ix: Instruction = construct_sketchy_ix(/* todo */);

// Construct a zipper instruction
let client: Client::new_with_options(
        Cluster::Mainnet,
        Rc::new(todo!("keypair")),
        CommitmentConfig::processed(),
    );
let program: Program = client.program(ZIPPER_PROGRAM_ID);
let zipper_ix: Instruction = program.request()
    .accounts(AccountZipper::zip_accounts(&[
            user.keypair.pubkey(),
            user.ata,
            user.ata2,
        ]))
    .args(zipper::instruction::Verify {
        balances: todo!(),
    })
    .instructions()
    .unwrap()
    .remove(0);

// Zip the sketchy instruction with the zipper instruction
let zipped_transaction = Transaction::new_signed_with_payer(
        &[sketchy_ix, zipper_ix],
        Some(&user.keypair.pubkey()),
        &[&*user.keypair],
        get_recent_blockhash(/* todo */),
    );

// If the balances drop under the specified `balances` in `Verify5`
// this transaction will fail
send_transaction(&zipped_transaction)
```
See the rust `spoof` test in `programs/zipper/tests/spoof.rs` for an end-to-end example on testnet.

# Pubkey
The testnet program ID is `Z1PctcuGZfcLVPWSCgi9nwhGNpvqH1AamRTEqSBzJoL`.
The program is yet to be deployed to Mainnet-Beta, but check back in within a week!.
