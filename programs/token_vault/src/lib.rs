use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_2022::{MintTo, TransferChecked};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
declare_id!("3pX5NKLru1UBDVckynWQxsgnJeUN3N1viy36Gk9TSn8d");
use anchor_spl::token_interface;
const ANCHOR_DISCRIMINATOR: usize = 8;
const TOKEN_DEMICAL: u8 = 6;
#[program]
pub mod token_example {

    use super::*;

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
        token_interface::transfer_checked(ix, amount, TOKEN_DEMICAL)?;

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
        token_interface::transfer_checked(ix, amount, TOKEN_DEMICAL)?;
        ctx.accounts.bookeeping_acc.quantity -= amount;

        Ok(())
    }
    pub fn initialize_token_subscription(
        ctx: Context<InitializeAccount>,
        subscription_price: u64,
        duration: u64,
    ) -> Result<()> {
        ctx.accounts.config.admin = ctx.accounts.owner.key();
        ctx.accounts.config.price = subscription_price;
        ctx.accounts.config.duration = duration;
        ctx.accounts.config.is_paused = false;
        Ok(())
    }
    pub fn set_price(ctx: Context<ChangePrice>, new_price: u64) -> Result<()> {
        ctx.accounts.config.price = new_price;
        Ok(())
    }
    pub fn subscribe_to_vault(ctx: Context<SubscribeToVault>) -> Result<()> {
        let trans_req = TransferChecked {
            from: ctx.accounts.user_ata.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
            to: ctx.accounts.vault_ata.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
        };
        let req = CpiContext::new(ctx.accounts.token_program.to_account_info(), trans_req);
        token_interface::transfer_checked(req, ctx.accounts.config.price, TOKEN_DEMICAL)?;

        let clock = Clock::get()?;

        ctx.accounts.subcription.owner = ctx.accounts.owner.key();
        let expiry_time = (ctx.accounts.config.duration + clock.unix_timestamp as u64) as i64;
        ctx.accounts.subcription.expires_at = expiry_time;
        emit!(SuccesfullSubscription {
            message: "success".to_string(),
            owner: ctx.accounts.owner.key(),
            expires_at: expiry_time,
        });
        Ok(())
    }
    pub fn renew_subscription(ctx: Context<SubscribeToVault>) -> Result<()> {
        let trans_req = TransferChecked {
            from: ctx.accounts.user_ata.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
            to: ctx.accounts.vault_ata.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
        };
        let req = CpiContext::new(ctx.accounts.token_program.to_account_info(), trans_req);
        token_interface::transfer_checked(req, ctx.accounts.config.price, TOKEN_DEMICAL)?;
        let clock = Clock::get()?;
        let current_time_unix = clock.unix_timestamp;
        let new_expity = if current_time_unix > ctx.accounts.subcription.expires_at {
            current_time_unix + ctx.accounts.config.duration as i64
        } else {
            ctx.accounts.subcription.expires_at + ctx.accounts.config.duration as i64
        };

        ctx.accounts.subcription.expires_at = new_expity;
        emit!(SuccesfullRenew {
            message: "success".to_string(),
            owner: ctx.accounts.owner.key(),
            new_expiry: new_expity
        });
        Ok(())
    }
    pub fn is_user_subcribed(ctx: Context<isUserSubscriptionValid>) -> Result<()> {
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;
        if current_time <= ctx.accounts.user_acc.expires_at {
            emit!(IsValidSubscription { is_valid: true });
        } else {
            emit!(IsValidSubscription { is_valid: false })
        }
        Ok(())
    }
}
#[derive(Accounts)]
pub struct isUserSubscriptionValid<'info> {
    pub owner: AccountInfo<'info>,
    #[account(
    seeds=[b"adsayan_mint"],
    bump)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        seeds=[b"subcription", owner.key().as_ref()],
        bump
    )]
    pub user_acc: Account<'info, Subscription>,
}
#[derive(Accounts)]
pub struct SubscribeToVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut,
        seeds=[b"adsayan_mint"],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(init_if_needed,
        payer=owner,
        associated_token::mint=mint,
        associated_token::authority=owner,
    )]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut,
    seeds=[b"authority"],
    bump)]
    ///CHECK: ok
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut,
    associated_token::mint=mint,
    associated_token::authority=vault_authority)]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(mut,
        seeds=[b"config"],
        bump
    )]
    pub config: Account<'info, ConfigOwner>,
    #[account(
        init_if_needed,
        payer=owner,
        space = Subscription::INIT_SPACE,
    seeds=[b"subscription", owner.key().as_ref()],
    bump)]
    pub subcription: Account<'info, Subscription>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
#[derive(Accounts)]
pub struct ChangePrice<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut,
    seeds=[b"authority"],
    bump,
)]
    /// CHECK: CPI use
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut,
    seeds=[b"adsayan_mint"],
    bump,
)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut,
    seeds=[b"config"], 
        bump,
        has_one=admin,
    )]
    pub config: Account<'info, ConfigOwner>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct InitializeAccount<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(init,
        payer=owner,
        space=ANCHOR_DISCRIMINATOR + ConfigOwner::INIT_SPACE,
        seeds=[b"config"],
        bump
    )]
    pub config: Account<'info, ConfigOwner>,

    #[account(
        seeds=[b"authority"], //why don't I need an init here?
        bump,
    )]
    ///CHECK: only for CPI
    pub vault_authority: UncheckedAccount<'info>,
    #[account(init_if_needed,
    seeds=[b"adsayan_mint"],
    payer=owner,
    bump,
    mint::decimals=6,
    mint::authority=vault_authority.key(),
    mint::freeze_authority=vault_authority.key()
)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(init,
    payer=owner,
    associated_token::mint=mint,
    associated_token::authority=vault_authority,
)]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct WithdrawFromVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut,
        seeds=[b"authority"],
        bump
    )]
    ///CHECK: only for CPI
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
    ///CHECK: only for CPI
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
pub struct MintToUser<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds=[b"authority"],
        bump
    )]
    ///CHECK: only for CPI
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

#[derive(InitSpace, Debug)]
#[account]
pub struct ConfigOwner {
    pub admin: Pubkey,
    pub price: u64,
    pub duration: u64,
    pub is_paused: bool,
}
#[derive(InitSpace, Debug)]
#[account]
pub struct Subscription {
    pub owner: Pubkey,
    pub expires_at: i64,
}

#[event]
pub struct SuccesfullSubscription {
    pub message: String,
    pub owner: Pubkey,
    pub expires_at: i64,
}
#[event]
pub struct SuccesfullRenew {
    pub message: String,
    pub new_expiry: i64,
    pub owner: Pubkey,
}
#[event]
pub struct IsValidSubscription {
    pub is_valid: bool,
}
