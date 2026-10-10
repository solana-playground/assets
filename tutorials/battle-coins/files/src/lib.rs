use anchor_lang::prelude::*;
use anchor_spl::metadata::mpl_token_metadata::types::DataV2;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{create_metadata_accounts_v3, CreateMetadataAccountsV3, Metadata},
    token::{burn, mint_to, Burn, Mint, MintTo, Token, TokenAccount},
};

declare_id!("EjzKEbp6Zu4tF5AiHcZKrtCNC8YtG3NjEXUWc5o96hjL");

const ADMIN_PUBKEY: Pubkey =
    solana_program::pubkey!("4JGEMfDYuBHQcmj9ardUjTRWV3kTeuFV62tuSTsY5vnN");
const MAX_HEALTH: u8 = 100;

#[program]
pub mod anchor_token {
    use super::*;

    pub fn create_mint(
        ctx: Context<CreateMint>,
        uri: String,
        name: String,
        symbol: String,
    ) -> Result<()> {
        let seeds = b"reward";
        let bump = ctx.bumps.reward_token_mint;
        let signer: &[&[&[u8]]] = &[&[seeds, &[bump]]];

        let data_v2 = DataV2 {
            name,
            symbol,
            uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata_account.to_account_info(),
                mint: ctx.accounts.reward_token_mint.to_account_info(),
                mint_authority: ctx.accounts.reward_token_mint.to_account_info(),
                update_authority: ctx.accounts.reward_token_mint.to_account_info(),
                payer: ctx.accounts.admin.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            signer,
        );

        create_metadata_accounts_v3(cpi_ctx, data_v2, true, true, None)?;
        Ok(())
    }

    pub fn init_player(ctx: Context<InitPlayer>) -> Result<()> {
        ctx.accounts.player_data.health = MAX_HEALTH;
        Ok(())
    }

    pub fn kill_enemy(ctx: Context<KillEnemy>) -> Result<()> {
        if ctx.accounts.player_data.health < 10 {
            return err!(ErrorCode::NotEnoughHealth);
        }
        ctx.accounts.player_data.health -= 10;

        let seeds = b"reward";
        let bump = ctx.bumps.reward_token_mint;
        let signer: &[&[&[u8]]] = &[&[seeds, &[bump]]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.reward_token_mint.to_account_info(),
                to: ctx.accounts.player_token_account.to_account_info(),
                authority: ctx.accounts.reward_token_mint.to_account_info(),
            },
            signer,
        );

        let amount = (1u64)
            .checked_mul(10u64.pow(ctx.accounts.reward_token_mint.decimals as u32))
            .unwrap();

        mint_to(cpi_ctx, amount)
    }

    pub fn heal(ctx: Context<Heal>) -> Result<()> {
        ctx.accounts.player_data.health = MAX_HEALTH;

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.reward_token_mint.to_account_info(),
                from: ctx.accounts.player_token_account.to_account_info(),
                authority: ctx.accounts.player.to_account_info(),
            },
        );

        let amount = (1u64)
            .checked_mul(10u64.pow(ctx.accounts.reward_token_mint.decimals as u32))
            .unwrap();

        burn(cpi_ctx, amount)
    }
}

#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(mut, address = ADMIN_PUBKEY)]
    pub admin: Signer<'info>,

    #[account(
        init,
        seeds = [b"reward"],
        bump,
        payer = admin,
        mint::decimals = 9,
        mint::authority = reward_token_mint,
    )]
    pub reward_token_mint: Account<'info, Mint>,

    /// CHECK: Metaplex PDA
    #[account(
        mut,
        seeds = [
            b"metadata",
            token_metadata_program.key().as_ref(),
            reward_token_mint.key().as_ref(),
        ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub metadata_account: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitPlayer<'info> {
    #[account(
        init,
        payer = player,
        space = 8 + 1, // Discriminator + u8 health
        seeds = [b"player", player.key().as_ref()],
        bump,
    )]
    pub player_data: Account<'info, PlayerData>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct KillEnemy<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(mut, seeds = [b"player", player.key().as_ref()], bump)]
    pub player_data: Account<'info, PlayerData>,
    #[account(
        init_if_needed,
        payer = player,
        associated_token::mint = reward_token_mint,
        associated_token::authority = player
    )]
    pub player_token_account: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"reward"], bump)]
    pub reward_token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Heal<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(mut, seeds = [b"player", player.key().as_ref()], bump)]
    pub player_data: Account<'info, PlayerData>,
    #[account(mut, associated_token::mint = reward_token_mint, associated_token::authority = player)]
    pub player_token_account: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"reward"], bump)]
    pub reward_token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[account]
pub struct PlayerData {
    pub health: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Not enough health")]
    NotEnoughHealth,
}
