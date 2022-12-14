use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use solana_security_txt::security_txt;

declare_id!("Z1PrGTgZp5Q1WKewjF4XaTW2nHvNxvbxs7qW8p9qz5U");

security_txt! {
    name: "Zipper",
    project_url: "http://github.com/cavemanloverboy/zipper",
    contacts: "caveycool@gmail.com",
    policy: "https://github.com/cavemanloverboy/zipper/blob/main/SECURITY.md"
}

#[program]
pub mod zipper {
    use super::*;

    pub fn verify(ctx: Context<VerifyAccounts>, balances: Vec<u64>) -> Result<()> {
        // Check that the number of accounts provided is correct
        require_eq!(
            ctx.remaining_accounts.len(),
            balances.len(),
            ZipperError::InvalidNumberOfAccountsOrBalances
        );

        // Check that all accounts provided are either token accounts or
        // system program accounts and extract balances
        let actual_balances: Vec<(u64, String)> = ctx
            .remaining_accounts
            .into_iter()
            .map(|acc| {
                if let Ok(token_account) = TokenAccount::try_deserialize(&mut &**acc.data.borrow())
                {
                    // Attempt to deserialize spl token account and get balance + mint
                    return Ok((
                        token_account.amount,
                        format!(
                            "spl addr {}, mint {}",
                            acc.key.to_string(),
                            token_account.mint
                        ),
                    ));
                } else if acc.owner == &System::id() {
                    // If system program account just retrieve lamports
                    Ok((acc.lamports(), format!("sol addr {}", acc.key.to_string())))
                } else {
                    // Neither SPL or System Program Account
                    Err(ZipperError::NonSOLOrSPLAccountProvided.into())
                }
            })
            .collect::<Result<Vec<(u64, String)>>>()
            .map_err(|_| ZipperError::NonSOLOrSPLAccountProvided)?;

        // Check Balances
        for i in 0..actual_balances.len() {
            if actual_balances[i].0 < balances[i] {
                panic!(
                    "expected {} >= {} for {}",
                    actual_balances[i].0, balances[i], actual_balances[i].1,
                );
            } else {
                msg!(
                    "expected {} >= {} for {}",
                    actual_balances[i].0,
                    balances[i],
                    actual_balances[i].1,
                )
            }
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct VerifyAccounts {}

pub struct AccountZipper;

impl AccountZipper {
    /// The accounts should be zipped in the order corresponding to the balances in `balances`
    ///
    /// e.g. using `zip_accounts(&[account1, account2])` with `balances = [balance1, balance2]`
    /// results in the checks
    /// -->
    ///     assert!(balance(account1) >= balance1)
    ///     assert!(balance(account2) >= balance2)
    pub fn zip_accounts(keys: &[Pubkey]) -> Vec<AccountMeta> {
        keys.into_iter()
            .map(|&pubkey| AccountMeta {
                pubkey,
                is_signer: false,
                is_writable: false,
            })
            .collect()
    }
}

#[error_code]
pub enum ZipperError {
    #[msg("number of SOL + SPL accounts does not match the number of expected_balances provided")]
    InvalidNumberOfAccountsOrBalances,
    #[msg("one of the accounts has a lower-than-expected balance")]
    InsufficientBalance,
    #[msg("an account that is not an spl account was provided as an additional account")]
    NonSOLOrSPLAccountProvided,
}
