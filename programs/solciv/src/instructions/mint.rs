use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3},
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};
// use mpl_token_metadata::id as mpl_token_metadata_id; // Updated import
// use mpl_token_metadata::state::DataV2; // Updated for compatibility with the latest version
// use mpl_token_metadata::{pda::find_metadata_account, types::DataV2};
use crate::errors::GameError;
use crate::state::Player;
use anchor_spl::metadata::Metadata;

pub fn mint_gems(ctx: Context<MintGems>) -> Result<()> {
    // Ensure the player has gems
    let gems = ctx.accounts.player_account.resources.gems as u64;
    if gems == 0 {
        return err!(GameError::NotEnoughGems);
    }

    // Calculate mint amount safely
    let amount = gems
        .checked_mul(1_000_000_000)
        .ok_or(GameError::CalculationOverflow)?;

    // Reset gems in player account
    ctx.accounts.player_account.resources.gems = 0;

    // PDA signer seeds
    let seeds = &[b"mint".as_ref(), &[ctx.bumps.mint]];
    let signer = [&seeds[..]];

    // Mint tokens to the destination account
    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                authority: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.destination.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
            &signer,
        ),
        amount,
    )?;

    Ok(())
}

pub fn create_gems(
    ctx: Context<CreateGems>,
    token_name: String,
    token_symbol: String,
    token_uri: String,
) -> Result<()> {
    msg!("Creating metadata account");

    // PDA signer seeds
    let seeds = &[b"mint".as_ref(), &[ctx.bumps.mint_account]];
    let signer_seeds = &[&seeds[..]];

    // Calculate the metadata account PDA
    let metadata_pda = Pubkey::find_program_address(
        &[
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            ctx.accounts.mint_account.key().as_ref(),
        ],
        &mpl_token_metadata::ID, //mpl_token_metadata_id(),
    )
    .0;

    // Verify the metadata account address
    if ctx.accounts.metadata_account.key() != metadata_pda {
        return err!(GameError::InvalidMetadataAccount);
    }

    let data_v2 = DataV2 {
        name: token_name,
        symbol: token_symbol,
        uri: token_uri,
        seller_fee_basis_points: 0,
        creators: None,
        collection: None,
        uses: None,
    };

    // CPI Context
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_metadata_program.to_account_info(),
        CreateMetadataAccountsV3 {
            metadata: ctx.accounts.metadata_account.to_account_info(),
            mint: ctx.accounts.mint_account.to_account_info(),
            mint_authority: ctx.accounts.mint_account.to_account_info(),
            update_authority: ctx.accounts.mint_account.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        }
    ).with_signer(signer_seeds);

    // Create the metadata account
    create_metadata_accounts_v3(
        cpi_ctx,
        data_v2,
        false, // Is mutable
        true,  // Update authority is signer
        None,  // Collection details
    )?;

    msg!("Token created successfully.");
    Ok(())
}

#[derive(Accounts)]
pub struct CreateGems<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        seeds = [b"mint"],
        bump,
        payer = payer,
        mint::decimals = 9,
        mint::authority = mint_account.key(),
        mint::freeze_authority = mint_account.key(),
    )]
    pub mint_account: Account<'info, Mint>,

    /// CHECK: Verified using PDA logic
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MintGems<'info> {
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = player,
        associated_token::mint = mint,
        associated_token::authority = owner,
    )]
    pub destination: Account<'info, TokenAccount>,

    /// CHECK: Verified off-chain
    pub owner: AccountInfo<'info>,

    #[account(mut, has_one = player)]
    pub player_account: Account<'info, Player>,

    #[account(mut)]
    pub player: Signer<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
