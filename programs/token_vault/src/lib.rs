use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Token, TokenAccount};
use anchor_spl::token_2022::MintTo;
use anchor_spl::token_interface::{Mint, TokenInterface};
declare_id!("3pX5NKLru1UBDVckynWQxsgnJeUN3N1viy36Gk9TSn8d");
use anchor_spl::token::Transfer;
use anchor_spl::token_interface;
const ANCHOR_DISCRIMINATOR: usize = 8;
const TOKEN_DECIAL: u64 = 6;
#[program]
pub mod token_example {

    use anchor_spl::token_2022::TransferChecked;

    use super::*;

    pub fn create_mint(ctx: Context<CreateMint>) -> Result<()> {
        Ok(())
    }
    pub fn mint_to_user(ctx: Context<MintToUser>, amount: u64) -> Result<()> {
        let acc = ctx.accounts;
        let ix = MintTo {
            mint: acc.mint.to_account_info(),
            to: acc.user_token_account.to_account_info(),
            authority: acc.vault_authority.to_account_info(),
        };
        let seeds: &[&[&[u8]]] = &[&[b"authority", &[ctx.bumps.vault_authority]]];
        let cpi_tx = CpiContext::new_with_signer(acc.token_program.to_account_info(), ix, seeds);
        token_interface::mint_to(cpi_tx, amount)?;
        Ok(())
    }
    //:TODO: add create vault_token_acc, struct already written

    pub fn deposit(ctx: Context<DepositToVault>, amount: u64) -> Result<()> {
        let from_acc = ctx.accounts.user_token_acc.to_account_info();
        let to_acc = ctx.accounts.vault_acc.to_account_info();
        let owner_permission_acc = ctx.accounts.owner.to_account_info();
        let mint = ctx.accounts.mint.to_account_info();
        let transfer_req = TransferChecked {
            from: from_acc,
            mint,
            to: to_acc,
            authority: owner_permission_acc,
        };
        let ix = CpiContext::new(ctx.accounts.token_program.to_account_info(), transfer_req);
        token_interface::transfer_checked(ix, amount, TOKEN_DECIAL as u8)?;

        ctx.accounts.data.owner = ctx.accounts.user_token_acc.key();
        ctx.accounts.data.quantity += amount;
        Ok(())
    }
    pub fn withdraw(ctx: Context<WithdrawFromVault>, amount: u64) -> Result<()> {
        let user_acc = &ctx.accounts.bookeeping_acc;
        if user_acc.quantity < amount {
            return err!(TokenError::NotEnoughFunds);
        }
        let from = ctx.accounts.vault_ata.to_account_info();
        let to = ctx.accounts.owner_ata.to_account_info();
        let mint = ctx.accounts.mint.to_account_info();
        let authority = ctx.accounts.vault_authority.to_account_info();
        let cpi_req = TransferChecked {
            from,
            to,
            mint,
            authority,
        };

        let seeds: &[&[&[u8]]] = &[&[b"authority", &[ctx.bumps.vault_authority]]];

        let ix = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_req,
            seeds,
        );
        token_interface::transfer_checked(ix, amount, TOKEN_DECIAL as u8)?;
        ctx.accounts.bookeeping_acc.quantity -= amount;

        Ok(())
    }
}
#[derive(Accounts)]
pub struct WithdrawFromVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut,
        seeds=[b"authority"],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut,
        seeds=[b"mint"],
        bump,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint=mint,
        associated_token::authority=owner
    )]
    pub owner_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut,
        associated_token::mint=mint,
        associated_token::authority=vault_authority
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(mut,
        seeds=[b"deposit_info", owner.key().as_ref()],
        bump
    )]
    pub bookeeping_acc: Account<'info, DepositeToken>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
#[derive(Accounts)]
pub struct DepositToVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
    mut,
    seeds=[b"authority"],
    bump
)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut,
    seeds=[b"mint"],
    bump
)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
    mut,
    associated_token::mint=mint,
    associated_token::authority=owner,
)]
    pub user_token_acc: InterfaceAccount<'info, TokenAccount>,
    #[account(
            mut,
            associated_token::mint=mint,
            associated_token::authority=vault_authority,
        )]
    pub vault_acc: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer=owner,
        space=ANCHOR_DISCRIMINATOR + DepositeToken::INIT_SPACE,
        seeds=[b"deposit_info", owner.key().as_ref()],
        bump
        )]
    pub data: Account<'info, DepositeToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct CreateVaultToken<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut,
        seeds=[b"mint"],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut,
    seeds=[b"authority"],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
    init,
    payer=owner,
    associated_token::mint=mint,
    associated_token::authority=vault_authority,
)]
    pub data: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        seeds=[b"authority"],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = signer,
        seeds=[b"mint"],
        bump,
        mint::decimals = 6,
        mint::authority = vault_authority,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct MintToUser<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds=[b"authority"],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds=[b"mint"],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    //owner's ADW
    #[account(
        init_if_needed,
        payer=owner,
        associated_token::mint=mint,
        associated_token::authority=owner,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(InitSpace, Debug)]
#[account]
pub struct DepositeToken {
    pub owner: Pubkey,
    pub quantity: u64,
}

#[error_code]
pub enum TokenError {
    #[msg("not enough founds")]
    NotEnoughFunds,
}
