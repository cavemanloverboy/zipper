use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::parse::{Parse, ParseStream};
struct TokenAccountInput {
    pub field_count: u64,
}

impl Parse for TokenAccountInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let count = input.parse::<syn::LitInt>()?;
        Ok(TokenAccountInput {
            field_count: count.base10_parse().unwrap(),
        })
    }
}

/// This builds all of the `Accounts` struct expected by Anchor's #[derive(Accounts)]
#[proc_macro_attribute]
pub fn token_account_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(attr as TokenAccountInput);
    let mut found_struct = false;
    item.into_iter()
        .map(|mut r| {
            match &mut r {
                // Look for struct
                &mut proc_macro::TokenTree::Ident(ref ident) if ident.to_string() == "struct" => {
                    // react on keyword "struct" so we don't randomly modify non-structs
                    found_struct = true;
                    r
                }

                // Append number to struct name
                &mut proc_macro::TokenTree::Ident(ref mut ident)
                    if ident.to_string() == "TokenAccounts" =>
                {
                    *ident = proc_macro::Ident::new(
                        &format!("{}{}", ident.to_string(), input.field_count),
                        proc_macro::Span::call_site(),
                    );
                    r
                }

                // Append field names to struct
                &mut proc_macro::TokenTree::Group(ref group)
                    // Find opening brace
                    if group.delimiter() == proc_macro::Delimiter::Brace
                        && found_struct == true =>
                {
                    let mut stream = proc_macro::TokenStream::new();
                    stream.extend(
                        (1..=input.field_count)
                            .fold(vec![], |mut state: Vec<proc_macro::TokenStream>, i| {
                                let instruction_name_str = format!("token_account_{}", i);
                                let instruction_name =
                                    Ident::new(&instruction_name_str, Span::call_site());
                                state.push(
                                    quote!(
                                        #[account()]
                                        pub #instruction_name: Box<Account<'info, TokenAccount>>,
                                    )
                                    .into(),
                                );
                                state
                            })
                            .into_iter(),
                    );
                    stream.extend(group.stream());
                    proc_macro::TokenTree::Group(proc_macro::Group::new(
                        proc_macro::Delimiter::Brace,
                        stream,
                    ))
                }
                _ => r,
            }
        })
        .collect()
}

struct InstructionInput {
    pub instruction_count: u64,
}

impl Parse for InstructionInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let count = input.parse::<syn::LitInt>()?;
        Ok(InstructionInput {
            instruction_count: count.base10_parse().unwrap(),
        })
    }
}

#[proc_macro_attribute]
/// This builds all of the instructions expected by Anchor's #[program]
pub fn verify(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(attr as InstructionInput);
    let mut found_mod = false;
    item.into_iter()
        .map(|mut r| {
            match &mut r {
                // Look for mod
                &mut proc_macro::TokenTree::Ident(ref ident) if ident.to_string() == "mod" => {
                    // react on keyword "mod" so we don't randomly modify non-mods
                    found_mod = true;
                    r
                }

                &mut proc_macro::TokenTree::Group(ref group)
                    if group.delimiter() == proc_macro::Delimiter::Brace && found_mod == true =>
                {
                    // Opening brackets for the mod
                    let mut stream = proc_macro::TokenStream::new();
                    stream.extend(
                        (0..=input.instruction_count)
                            .fold(vec![], |mut state: Vec<proc_macro::TokenStream>, i| {
                                let instruction_name_str = format!("verify_{}", i);
                                let instruction_name =
                                    Ident::new(&instruction_name_str, Span::call_site());
                                let accounts_struct_name = format!("TokenAccounts{}", i);
                                let accounts_struct = Ident::new(&accounts_struct_name, Span::call_site());
                                let handler_name = format!("handler_{}", i);
                                let handler = Ident::new(&handler_name, Span::call_site());
                                let ip1: usize = i as usize + 1;
                                // push a method for each one
                                state.push(
                                    quote!{
                                        pub fn #instruction_name(ctx: Context<#accounts_struct>, balances: [u64; #ip1]) -> Result<()> {
                                            #handler(ctx, balances)
                                        }
                                    }
                                    .into()
                                );
                                state
                            })
                            .into_iter(),
                    );
                    stream.extend(group.stream());
                    proc_macro::TokenTree::Group(proc_macro::Group::new(
                        proc_macro::Delimiter::Brace,
                        stream,
                    ))
                }
                _ => r,
            }
        })
        .collect()
}
