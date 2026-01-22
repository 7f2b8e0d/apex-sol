use anchor_lang::prelude::*;
// 导入系统程序转账所需的库
use anchor_lang::system_program::{Transfer, transfer};

declare_id!("CpYfKERvMg4xWbj7wH67XyBGFCZNsRq8Q7rFgrcvfSAy");

#[program]
pub mod apex_sol_history {
    use super::*;

    pub fn initialize(ctx: Context<InitializePool>) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        pool.sol_reserve = 0;
        // 假设初始给池子 1,000,000 个代币用于兑换
        pool.token_reserve = 1_000_000; 
        msg!("Pool Initialized with Tokens!");
        Ok(())
    }

    pub fn swap(ctx: Context<Swap>, amount_in: u64) -> Result<()> {
        // 1. 安全校验：输入必须大于 0 (Check if input is valid)
        require!(amount_in > 0, ErrorCode::AmountTooLow);
        
        let pool = &mut ctx.accounts.pool;
        
        // 2. 安全校验：池子代币是否足够 (Check if pool has enough liquidity)
        require!(pool.token_reserve >= amount_in, ErrorCode::InsufficientLiquidity);

        // --- 核心步骤：真实转账 (Real SOL Transfer via CPI) ---
        // 重点标注：CPI (Cross-Program Invocation)
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: pool.to_account_info(),
            },
        );
        // 执行转账，如果用户钱包钱不够，这里会报错并中断
        transfer(cpi_context, amount_in)?;

        // 3. 更新账本状态 (Update reserves)
        pool.sol_reserve += amount_in;
        let amount_out = amount_in; 
        pool.token_reserve -= amount_out;

        // 4. 抛出实时事件 (Emit Event)
        let clock = Clock::get()?;
        emit!(SwapEvent {
            user: ctx.accounts.user.key(),
            amount_in,
            amount_out,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init, 
        payer = admin, 
        space = 8 + 16, 
        seeds = [b"global-pool"], 
        bump
    )]
    pub pool: Account<'info, PoolState>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut, seeds = [b"global-pool"], bump)]
    pub pool: Account<'info, PoolState>,
    #[account(mut)]
    pub user: Signer<'info>,
    // 重点标注：必须加上系统程序引用 (Must include System Program)
    pub system_program: Program<'info, System>,
}

#[account]
pub struct PoolState {
    pub sol_reserve: u64,
    pub token_reserve: u64,
}

#[event]
pub struct SwapEvent {
    pub user: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub timestamp: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero.")]
    AmountTooLow,
    #[msg("Not enough tokens in the pool.")]
    InsufficientLiquidity,
}