use std::rc::Rc;

use anchor_client::{
    solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig},
    solana_sdk::{
        commitment_config::CommitmentConfig,
        hash::Hash,
        instruction::Instruction,
        native_token::LAMPORTS_PER_SOL,
        signature::{read_keypair_file, Keypair},
        signer::Signer,
        system_transaction,
        transaction::Transaction,
    },
    Client, Cluster, Program,
};
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::{spl_token::instruction::transfer, Mint, ID as TOKEN_PROGRAM_ID};
use anyhow::Result;
use rand::rngs::OsRng;
use zipper::{AccountZipper, ID as PROGRAM_ID};

const DEMO_TOKEN_DECIMALS: u8 = 6;
const ONE_DEMO_TOKEN: u64 = 10_u64.pow(DEMO_TOKEN_DECIMALS as u32);

#[test]
fn spoof() {
    // Get dev and mint key.
    let dev_key: Keypair = read_keypair_file(&*shellexpand::tilde("../../dev_key.json"))
        .expect("Example requires a keypair file");
    let mint_key: Keypair = read_keypair_file(&*shellexpand::tilde("../../mint_key.json"))
        .expect("Example requires a keypair file");
    let mint_key2: Keypair = read_keypair_file(&*shellexpand::tilde("../../mint_key2.json"))
        .expect("Example requires a keypair file");

    // Get client, program, and rpc client
    let client: Client = Client::new_with_options(
        std::env::args()
            .nth(1)
            .map(|x| match x.as_str() {
                "m" => Cluster::Mainnet,
                "t" => Cluster::Testnet,
                "l" => Cluster::Localnet,
                c => {
                    println!("unknown cluster '{}': falling back to localnet", c);
                    Cluster::Localnet
                }
            })
            .unwrap_or(Cluster::Localnet),
        Rc::new(Keypair::from_bytes(dev_key.to_bytes().as_ref()).unwrap()),
        CommitmentConfig::processed(),
    );
    let program: Program = client.program(PROGRAM_ID);
    let solana_client: RpcClient = program.rpc();

    // Initialize mint accounts if needed
    if let Ok(sig) = initialize_mint_account(&dev_key, &mint_key, &solana_client) {
        println!("initialize token mint tx signature: {}", sig);
    }
    if let Ok(sig) = initialize_mint_account(&dev_key, &mint_key2, &solana_client) {
        println!("initialize token mint tx signature: {}", sig);
    }

    // Get funded user and rugger (two token accounts)
    let user: User = get_funded_user(&dev_key, &mint_key, &mint_key2, &solana_client)
        .expect("failed to get funded user");
    let rugger: User = get_funded_user(&dev_key, &mint_key, &mint_key2, &solana_client)
        .expect("failed to get funded rugger");

    println!("SOL Balances:");
    println!(
        "    dev: {}",
        solana_client.get_balance(&dev_key.pubkey()).unwrap()
    );
    println!(
        "    usr: {}",
        solana_client.get_balance(&user.keypair.pubkey()).unwrap()
    );
    println!(
        "    rug: {}",
        solana_client.get_balance(&rugger.keypair.pubkey()).unwrap()
    );
    println!("SPL1 Balances:");
    println!(
        "    usr: {}",
        solana_client
            .get_token_account_balance(&user.ata)
            .unwrap()
            .amount
    );
    println!(
        "    rug: {}",
        solana_client
            .get_token_account_balance(&rugger.ata)
            .unwrap()
            .amount
    );
    println!("SPL2 Balances:");
    println!(
        "    usr: {}",
        solana_client
            .get_token_account_balance(&user.ata2)
            .unwrap()
            .amount
    );
    println!(
        "    rug: {}",
        solana_client
            .get_token_account_balance(&rugger.ata2)
            .unwrap()
            .amount
    );

    // Some instruction that transfers one demo token
    let transfer_instruction = transfer(
        &TOKEN_PROGRAM_ID,
        &user.ata, // source
        &rugger.ata,
        &user.keypair.pubkey(),
        &[&user.keypair.pubkey()],
        ONE_DEMO_TOKEN,
    )
    .unwrap();
    let recent_blockhash: Hash = solana_client
        .get_latest_blockhash()
        .expect("failed to get latest blockhash");
    let transaction: Transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&user.keypair.pubkey()),
        &[&*user.keypair],
        recent_blockhash,
    );

    // We will simulate the tx simulation, lol
    const INIT_SOL_BALANCE: u64 = LAMPORTS_PER_SOL / 100;
    const COST_OF_SPL_INIT: u64 = 4_088_560;
    const TX_FEE: u64 = 5_000;
    let simulation_of_simulation = |_tx: Transaction| {
        [
            INIT_SOL_BALANCE - COST_OF_SPL_INIT - TX_FEE,
            100 * ONE_DEMO_TOKEN - ONE_DEMO_TOKEN, // we expect to spend one spl1 token
            100 * ONE_DEMO_TOKEN,                  // we expect to spend no spl2 token
        ]
    };
    let simulated_post_balances: [u64; 3] = simulation_of_simulation(transaction);
    println!("simulated transaction");

    // Now, we simluate a spoof that changes the tx to one that invokes a larger transfer
    // In practice, this would be constructed within the runtime, but is simulated here.
    let rug_instruction = transfer(
        &TOKEN_PROGRAM_ID,
        &user.ata, // source
        &rugger.ata,
        &user.keypair.pubkey(),
        &[&user.keypair.pubkey()],
        100 * ONE_DEMO_TOKEN,
    )
    .unwrap();
    let recent_blockhash: Hash = solana_client
        .get_latest_blockhash()
        .expect("failed to get latest blockhash");

    // However, since we zip our backpack, this transaction will fail
    // We only need to zip the accounts that were passed into the previous instruction
    let zipper: Instruction = program
        .request()
        .accounts(AccountZipper::zip_accounts(&[
            user.keypair.pubkey(),
            user.ata,
            user.ata2,
        ]))
        .args(zipper::instruction::Verify {
            balances: simulated_post_balances.to_vec(),
        })
        .instructions()
        .unwrap()
        .remove(0);
    let zipped_transaction: Transaction = Transaction::new_signed_with_payer(
        &[rug_instruction, zipper],
        Some(&user.keypair.pubkey()),
        &[&*user.keypair],
        recent_blockhash,
    );
    println!("zipped transaction");

    // This fails!
    match solana_client.send_transaction_with_config(
        &zipped_transaction,
        RpcSendTransactionConfig {
            skip_preflight: true,
            ..RpcSendTransactionConfig::default()
        },
    ) {
        Ok(sig) => println!("failed sig {sig:#?}"),
        Err(e) => println!("{e:#?}"),
    }
    #[allow(deprecated)]
    std::thread::sleep_ms(1000);

    println!("SOL Balances:");
    println!(
        "    dev: {}",
        solana_client.get_balance(&dev_key.pubkey()).unwrap()
    );
    println!(
        "    usr: {}",
        solana_client.get_balance(&user.keypair.pubkey()).unwrap()
    );
    println!(
        "    rug: {}",
        solana_client.get_balance(&rugger.keypair.pubkey()).unwrap()
    );
    println!("SPL1 Balances:");
    println!(
        "    usr: {}",
        solana_client
            .get_token_account_balance(&user.ata)
            .unwrap()
            .amount
    );
    println!(
        "    rug: {}",
        solana_client
            .get_token_account_balance(&rugger.ata)
            .unwrap()
            .amount
    );
    println!("SPL2 Balances:");
    println!(
        "    usr: {}",
        solana_client
            .get_token_account_balance(&user.ata2)
            .unwrap()
            .amount
    );
    println!(
        "    rug: {}",
        solana_client
            .get_token_account_balance(&rugger.ata2)
            .unwrap()
            .amount
    );

    // Assert user did not get rugged
    assert!(
        solana_client.get_balance(&user.keypair.pubkey()).unwrap()
            >= LAMPORTS_PER_SOL / 100 - COST_OF_SPL_INIT - TX_FEE,
    );
    assert_eq!(
        solana_client
            .get_token_account_balance(&user.ata)
            .unwrap()
            .amount
            .parse::<u64>()
            .unwrap(),
        100 * ONE_DEMO_TOKEN,
    );
    assert_eq!(
        solana_client
            .get_token_account_balance(&user.ata2)
            .unwrap()
            .amount
            .parse::<u64>()
            .unwrap(),
        100 * ONE_DEMO_TOKEN,
    );
    println!("\nSuccess: all balances are correct! Rug Prevented!\n");

    // Repeat the process, adding a random sol account as the second account
    // (suppose the rugger was going to trade sol,
    //  e.g. that this was an escrow or swap program of sorts)

    // Now, we simluate a spoof that changes the tx to one that invokes a larger transfer
    // In practice, this would be constructed within the runtime, but is simulated here.
    let rug_instruction = transfer(
        &TOKEN_PROGRAM_ID,
        &user.ata, // source
        &rugger.ata,
        &user.keypair.pubkey(),
        &[&user.keypair.pubkey()],
        100 * ONE_DEMO_TOKEN,
    )
    .unwrap();
    let recent_blockhash: Hash = solana_client
        .get_latest_blockhash()
        .expect("failed to get latest blockhash");

    // However, since we zip our backpack, this transaction will fail
    // We only need to zip the accounts that were passed into the previous instruction
    let zipper: Instruction = program
        .request()
        .accounts(AccountZipper::zip_accounts(&[
            user.keypair.pubkey(),
            rugger.keypair.pubkey(), // adding this one!
            user.ata,
            user.ata2,
        ]))
        .args(zipper::instruction::Verify {
            balances: {
                let mut balances = simulated_post_balances.to_vec();
                // The first tx failed
                balances[0] -= TX_FEE;
                // add the rugger sol balance,
                // should be more than balance[0]
                balances.insert(1, balances[0]);
                balances
            },
        })
        .instructions()
        .unwrap()
        .remove(0);
    let zipped_transaction: Transaction = Transaction::new_signed_with_payer(
        &[rug_instruction, zipper],
        Some(&user.keypair.pubkey()),
        &[&*user.keypair],
        recent_blockhash,
    );
    println!("zipped transaction");

    // This fails!
    match solana_client.send_transaction_with_config(
        &zipped_transaction,
        RpcSendTransactionConfig {
            skip_preflight: true,
            ..RpcSendTransactionConfig::default()
        },
    ) {
        Ok(sig) => println!("failed sig {sig:#?}"),
        Err(e) => println!("{e:#?}"),
    }
    #[allow(deprecated)]
    std::thread::sleep_ms(1000);

    println!("SOL Balances:");
    println!(
        "    dev: {}",
        solana_client.get_balance(&dev_key.pubkey()).unwrap()
    );
    println!(
        "    usr: {}",
        solana_client.get_balance(&user.keypair.pubkey()).unwrap()
    );
    println!(
        "    rug: {}",
        solana_client.get_balance(&rugger.keypair.pubkey()).unwrap()
    );
    println!("SPL1 Balances:");
    println!(
        "    usr: {}",
        solana_client
            .get_token_account_balance(&user.ata)
            .unwrap()
            .amount
    );
    println!(
        "    rug: {}",
        solana_client
            .get_token_account_balance(&rugger.ata)
            .unwrap()
            .amount
    );
    println!("SPL2 Balances:");
    println!(
        "    usr: {}",
        solana_client
            .get_token_account_balance(&user.ata2)
            .unwrap()
            .amount
    );
    println!(
        "    rug: {}",
        solana_client
            .get_token_account_balance(&rugger.ata2)
            .unwrap()
            .amount
    );

    // Assert user did not get rugged
    assert!(
        solana_client.get_balance(&user.keypair.pubkey()).unwrap()
            >= LAMPORTS_PER_SOL / 100 - COST_OF_SPL_INIT - 2 * TX_FEE,
    );
    assert_eq!(
        solana_client
            .get_token_account_balance(&user.ata)
            .unwrap()
            .amount
            .parse::<u64>()
            .unwrap(),
        100 * ONE_DEMO_TOKEN,
    );
    assert_eq!(
        solana_client
            .get_token_account_balance(&user.ata2)
            .unwrap()
            .amount
            .parse::<u64>()
            .unwrap(),
        100 * ONE_DEMO_TOKEN,
    );
    println!("\nSuccess: all balances are correct! Rug Prevented Again!\n");

    // Repeat the process, one final time, now with no rug

    // Don't rug this time
    let nonrug_instruction = transfer(
        &TOKEN_PROGRAM_ID,
        &user.ata, // source
        &rugger.ata,
        &user.keypair.pubkey(),
        &[&user.keypair.pubkey()],
        ONE_DEMO_TOKEN,
    )
    .unwrap();
    let recent_blockhash: Hash = solana_client
        .get_latest_blockhash()
        .expect("failed to get latest blockhash");

    // However, since we zip our backpack, this transaction will fail
    // We only need to zip the accounts that were passed into the previous instruction
    let zipper: Instruction = program
        .request()
        .accounts(AccountZipper::zip_accounts(&[
            user.keypair.pubkey(),
            user.ata,
            user.ata2,
        ]))
        .args(zipper::instruction::Verify {
            balances: {
                let mut balances = simulated_post_balances.to_vec();
                // The first and second tx failed
                balances[0] -= 2 * TX_FEE;
                balances
            },
        })
        .instructions()
        .unwrap()
        .remove(0);
    let zipped_transaction: Transaction = Transaction::new_signed_with_payer(
        &[nonrug_instruction, zipper],
        Some(&user.keypair.pubkey()),
        &[&*user.keypair],
        recent_blockhash,
    );
    println!("zipped transaction");

    // This succeds!
    match solana_client.send_transaction_with_config(
        &zipped_transaction,
        RpcSendTransactionConfig {
            skip_preflight: true,
            ..RpcSendTransactionConfig::default()
        },
    ) {
        Ok(sig) => println!("success sig {sig:#?}"),
        Err(e) => println!("{e:#?}"),
    }
    #[allow(deprecated)]
    std::thread::sleep_ms(1000);

    println!("SOL Balances:");
    println!(
        "    dev: {}",
        solana_client.get_balance(&dev_key.pubkey()).unwrap()
    );
    println!(
        "    usr: {}",
        solana_client.get_balance(&user.keypair.pubkey()).unwrap()
    );
    println!(
        "    rug: {}",
        solana_client.get_balance(&rugger.keypair.pubkey()).unwrap()
    );
    println!("SPL1 Balances:");
    println!(
        "    usr: {}",
        solana_client
            .get_token_account_balance(&user.ata)
            .unwrap()
            .amount
    );
    println!(
        "    tgt: {}",
        solana_client
            .get_token_account_balance(&rugger.ata)
            .unwrap()
            .amount
    );
    println!("SPL2 Balances:");
    println!(
        "    usr: {}",
        solana_client
            .get_token_account_balance(&user.ata2)
            .unwrap()
            .amount
    );
    println!(
        "    tgt: {}",
        solana_client
            .get_token_account_balance(&rugger.ata2)
            .unwrap()
            .amount
    );

    // Assert xfer is successful
    assert!(
        solana_client.get_balance(&user.keypair.pubkey()).unwrap()
            >= LAMPORTS_PER_SOL / 100 - COST_OF_SPL_INIT - 3 * TX_FEE,
    );
    assert_eq!(
        solana_client
            .get_token_account_balance(&user.ata)
            .unwrap()
            .amount
            .parse::<u64>()
            .unwrap(),
        99 * ONE_DEMO_TOKEN,
    );
    assert_eq!(
        solana_client
            .get_token_account_balance(&user.ata2)
            .unwrap()
            .amount
            .parse::<u64>()
            .unwrap(),
        100 * ONE_DEMO_TOKEN,
    );
    println!("\nSuccess: all balances are correct!\n");
}

fn get_funded_user(
    dev_key: &Keypair,
    mint_key: &Keypair,
    mint_key2: &Keypair,
    solana_client: &RpcClient,
) -> Result<User> {
    // Generate a new keypair
    let user = Keypair::generate(&mut OsRng);

    // Fund the keypair from the dev wallet with sol
    let fund_with_sol_tx: Transaction = system_transaction::transfer(
        dev_key,
        &user.pubkey(),
        LAMPORTS_PER_SOL / 100,
        solana_client
            .get_latest_blockhash()
            .expect("failed to get lastest blockhash"),
    );
    println!(
        "fund_with_sol_tx signature: {}",
        solana_client
            .send_and_confirm_transaction(&fund_with_sol_tx)
            .expect("failed to fund user with sol")
    );
    assert_eq!(
        solana_client
            .get_balance(&user.pubkey())
            .expect("failed to get balance"),
        LAMPORTS_PER_SOL / 100,
    );
    drop(fund_with_sol_tx);

    // Create user token account
    let user_ata: Pubkey = spl_associated_token_account::get_associated_token_address(
        &user.pubkey(),
        &mint_key.pubkey(),
    );
    let spl_create_account_ix: Instruction =
        spl_associated_token_account::instruction::create_associated_token_account(
            &user.pubkey(),
            &user.pubkey(),
            &mint_key.pubkey(),
        );
    let create_spl_account_tx: Transaction = Transaction::new_signed_with_payer(
        &[spl_create_account_ix],
        Some(&user.pubkey()),
        &[&user],
        solana_client
            .get_latest_blockhash()
            .expect("failed to get lastest blockhash"),
    );
    println!(
        "create_spl_account_tx signature: {}",
        solana_client
            .send_and_confirm_transaction(&create_spl_account_tx)
            .expect("failed to create spl account ")
    );
    drop(create_spl_account_tx);

    // Create user token account2
    let user_ata2: Pubkey = spl_associated_token_account::get_associated_token_address(
        &user.pubkey(),
        &mint_key2.pubkey(),
    );
    let spl_create_account_ix2: Instruction =
        spl_associated_token_account::instruction::create_associated_token_account(
            &user.pubkey(),
            &user.pubkey(),
            &mint_key2.pubkey(),
        );
    let create_spl_account_tx2: Transaction = Transaction::new_signed_with_payer(
        &[spl_create_account_ix2],
        Some(&user.pubkey()),
        &[&user],
        solana_client
            .get_latest_blockhash()
            .expect("failed to get lastest blockhash"),
    );
    println!(
        "create_spl_account_tx signature: {}",
        solana_client
            .send_and_confirm_transaction(&create_spl_account_tx2)
            .expect("failed to create spl account ")
    );
    drop(create_spl_account_tx2);

    // Ensure account properties are okay
    let user_token_account = solana_client
        .get_token_account(&user_ata)
        .expect("failed to retrieve user token account")
        .expect("expecting user account to exist");
    assert_eq!(
        &user.pubkey().to_string(),
        &user_token_account.owner,
        "incorrect ata owner"
    );

    // Fund first token account by minting tokens
    let token_mint_ix: Instruction = anchor_spl::token::spl_token::instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        &mint_key.pubkey(),
        &user_ata,
        &dev_key.pubkey(),
        &[&dev_key.pubkey()],
        100 * ONE_DEMO_TOKEN,
    )
    .expect("unable to create mint transaction");
    let fund_with_spl_tx: Transaction = Transaction::new_signed_with_payer(
        &[token_mint_ix],
        Some(&dev_key.pubkey()),
        &[dev_key],
        solana_client
            .get_latest_blockhash()
            .expect("failed to get lastest blockhash"),
    );
    println!(
        "fund_with_spl_tx signature: {}",
        solana_client
            .send_and_confirm_transaction(&fund_with_spl_tx)
            .expect("failed to create spl account ")
    );
    drop(fund_with_spl_tx);

    // Fund second token account by minting tokens
    let token_mint_ix2: Instruction = anchor_spl::token::spl_token::instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        &mint_key2.pubkey(),
        &user_ata2,
        &dev_key.pubkey(),
        &[&dev_key.pubkey()],
        100 * ONE_DEMO_TOKEN,
    )
    .expect("unable to create mint transaction");
    let fund_with_spl_tx2: Transaction = Transaction::new_signed_with_payer(
        &[token_mint_ix2],
        Some(&dev_key.pubkey()),
        &[dev_key],
        solana_client
            .get_latest_blockhash()
            .expect("failed to get lastest blockhash"),
    );
    println!(
        "fund_with_spl_tx2 signature: {}",
        solana_client
            .send_and_confirm_transaction(&fund_with_spl_tx2)
            .expect("failed to create spl account ")
    );
    drop(fund_with_spl_tx2);

    Ok(User {
        keypair: Rc::new(user),
        ata: user_ata,
        ata2: user_ata2,
    })
}

/// This allow(unused_must_use) makes this function idempotent & infallible with a valid dev environment
#[allow(unused_must_use)]
fn initialize_mint_account(
    dev_key: &Keypair,
    mint_key: &Keypair,
    solana_client: &RpcClient,
) -> Result<String, anchor_client::solana_client::client_error::ClientError> {
    // Create transaction with single spl mint instruction
    let pay_rent_and_create_account_ix: Instruction =
        anchor_client::solana_sdk::system_instruction::create_account(
            &dev_key.pubkey(),
            &mint_key.pubkey(),
            solana_client.get_minimum_balance_for_rent_exemption(Mint::LEN)?,
            Mint::LEN as u64,
            &TOKEN_PROGRAM_ID,
        );
    let initialize_mint_account_ix: Instruction =
        anchor_spl::token::spl_token::instruction::initialize_mint(
            &TOKEN_PROGRAM_ID,
            &mint_key.pubkey(),
            &dev_key.pubkey(),
            None,
            DEMO_TOKEN_DECIMALS,
        )
        .expect("failed to create initialize mint account instruction");
    let spl_mint_tx = Transaction::new_signed_with_payer(
        &[pay_rent_and_create_account_ix, initialize_mint_account_ix],
        Some(&dev_key.pubkey()),
        &[dev_key, mint_key],
        solana_client.get_latest_blockhash()?,
    );

    // Send and confirm transaction, and get signature
    let signature = solana_client.send_and_confirm_transaction(&spl_mint_tx);
    signature.map(|s| s.to_string())
}

struct User {
    keypair: Rc<Keypair>,
    ata: Pubkey,
    ata2: Pubkey,
}
