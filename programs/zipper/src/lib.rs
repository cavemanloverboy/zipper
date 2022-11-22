use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use seq_macro::seq;
use zipper_macros::{token_account_struct, verify};

declare_id!("Z1PctcuGZfcLVPWSCgi9nwhGNpvqH1AamRTEqSBzJoL");

#[verify(31)] // builds 32 instructions, 0 (SOL only) to 31 token accounts
#[program]
pub mod zipper {
    use super::*;
}

macro_rules! accounts_struct(
    ($num:literal) => {
        #[token_account_struct($num)]
        #[derive(Accounts)]
        pub struct TokenAccounts<'info> {
            /// CHECK: read-only
            #[account()]
            user: AccountInfo<'info>,
        }
    }
);
seq!(N in 0..=31 {
    accounts_struct!(N);
});

seq!(N in 0..=31 {
    /// The first entry of balances is for SOL balance
    fn handler_~N(ctx: Context<TokenAccounts~N>, balances: [u64; N+1]) -> Result<()> {
        // Check SOL balance
        assert!(**ctx.accounts.user.lamports.borrow() >= balances[0], "expected at least {} for {}, got {}", balances[0], ctx.accounts.user.key(), ctx.accounts.user.lamports.borrow());
        seq!(M in 1..=N {
            // Check Mth token account
            assert!(ctx.accounts.token_account_~M.amount >= balances[M], "expected at least {} for {}, got {}", balances[M], ctx.accounts.token_account_~M.key(), ctx.accounts.token_account_~M.amount);
        });
        Ok(())
    }
});
