use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("A11st1111111111111111111111111111111111111");

#[program]
pub mod agentlist {
    use super::*;

    /// Register an agent with skills and stake
    pub fn register_agent(
        ctx: Context<RegisterAgent>,
        name: String,
        skills: Vec<String>,
        bio: String,
    ) -> Result<()> {
        let agent = &mut ctx.accounts.agent_profile;
        agent.authority = ctx.accounts.authority.key();
        agent.name = name;
        agent.skills = skills;
        agent.bio = bio;
        agent.reputation = 0;
        agent.tasks_completed = 0;
        agent.tasks_posted = 0;
        agent.total_earned = 0;
        agent.total_paid = 0;
        agent.registered_at = Clock::get()?.unix_timestamp;
        agent.bump = ctx.bumps.agent_profile;
        
        Ok(())
    }

    /// Post a task with SOL escrow
    pub fn post_task(
        ctx: Context<PostTask>,
        description: String,
        budget: u64,
        deadline: i64,
        required_skills: Vec<String>,
    ) -> Result<()> {
        let task = &mut ctx.accounts.task;
        let poster = &mut ctx.accounts.poster_profile;
        
        // Transfer SOL to escrow
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.poster.key(),
            &ctx.accounts.escrow.key(),
            budget,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.poster.to_account_info(),
                ctx.accounts.escrow.to_account_info(),
            ],
        )?;
        
        task.poster = ctx.accounts.poster.key();
        task.description = description;
        task.budget = budget;
        task.deadline = deadline;
        task.required_skills = required_skills;
        task.status = TaskStatus::Open;
        task.claimer = None;
        task.posted_at = Clock::get()?.unix_timestamp;
        task.bump = ctx.bumps.task;
        
        poster.tasks_posted += 1;
        
        Ok(())
    }

    /// Claim an open task
    pub fn claim_task(ctx: Context<ClaimTask>) -> Result<()> {
        let task = &mut ctx.accounts.task;
        
        require!(
            task.status == TaskStatus::Open,
            AgentListError::TaskNotOpen
        );
        
        require!(
            Clock::get()?.unix_timestamp < task.deadline,
            AgentListError::TaskExpired
        );
        
        task.status = TaskStatus::InProgress;
        task.claimer = Some(ctx.accounts.claimer.key());
        task.claimed_at = Some(Clock::get()?.unix_timestamp);
        
        Ok(())
    }

    /// Submit task completion (claimer)
    pub fn submit_completion(
        ctx: Context<SubmitCompletion>,
        proof_uri: String,
        summary: String,
    ) -> Result<()> {
        let task = &mut ctx.accounts.task;
        
        require!(
            task.status == TaskStatus::InProgress,
            AgentListError::TaskNotInProgress
        );
        
        require!(
            task.claimer == Some(ctx.accounts.claimer.key()),
            AgentListError::NotClaimer
        );
        
        task.status = TaskStatus::PendingApproval;
        task.proof_uri = Some(proof_uri);
        task.summary = Some(summary);
        task.submitted_at = Some(Clock::get()?.unix_timestamp);
        
        Ok(())
    }

    /// Approve task and release escrow (poster)
    pub fn approve_task(ctx: Context<ApproveTask>) -> Result<()> {
        let task = &mut ctx.accounts.task;
        
        require!(
            task.status == TaskStatus::PendingApproval,
            AgentListError::TaskNotPendingApproval
        );
        
        require!(
            task.poster == ctx.accounts.poster.key(),
            AgentListError::NotPoster
        );
        
        let claimer_key = task.claimer.ok_or(AgentListError::NoClaimer)?;
        
        // Transfer from escrow to claimer
        **ctx.accounts.escrow.to_account_info().try_borrow_mut_lamports()? -= task.budget;
        **ctx.accounts.claimer.try_borrow_mut_lamports()? += task.budget;
        
        task.status = TaskStatus::Completed;
        task.completed_at = Some(Clock::get()?.unix_timestamp);
        
        // Update reputations
        let claimer_profile = &mut ctx.accounts.claimer_profile;
        claimer_profile.tasks_completed += 1;
        claimer_profile.total_earned += task.budget;
        claimer_profile.reputation += 10; // Simple reputation for v1
        
        let poster_profile = &mut ctx.accounts.poster_profile;
        poster_profile.total_paid += task.budget;
        
        Ok(())
    }

    /// Dispute a task (poster rejects completion)
    pub fn dispute_task(ctx: Context<DisputeTask>, reason: String) -> Result<()> {
        let task = &mut ctx.accounts.task;
        
        require!(
            task.status == TaskStatus::PendingApproval,
            AgentListError::TaskNotPendingApproval
        );
        
        require!(
            task.poster == ctx.accounts.poster.key(),
            AgentListError::NotPoster
        );
        
        task.status = TaskStatus::Disputed;
        task.dispute_reason = Some(reason);
        
        // In v1, disputed tasks return funds to poster
        // In v2, add arbitration system
        **ctx.accounts.escrow.to_account_info().try_borrow_mut_lamports()? -= task.budget;
        **ctx.accounts.poster.try_borrow_mut_lamports()? += task.budget;
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct RegisterAgent<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + AgentProfile::INIT_SPACE,
        seeds = [b"agent", authority.key().as_ref()],
        bump
    )]
    pub agent_profile: Account<'info, AgentProfile>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(description: String)]
pub struct PostTask<'info> {
    #[account(
        init,
        payer = poster,
        space = 8 + Task::INIT_SPACE,
        seeds = [b"task", poster.key().as_ref(), &poster_profile.tasks_posted.to_le_bytes()],
        bump
    )]
    pub task: Account<'info, Task>,
    
    /// CHECK: Escrow PDA to hold task funds
    #[account(
        mut,
        seeds = [b"escrow", task.key().as_ref()],
        bump
    )]
    pub escrow: AccountInfo<'info>,
    
    #[account(
        mut,
        seeds = [b"agent", poster.key().as_ref()],
        bump = poster_profile.bump
    )]
    pub poster_profile: Account<'info, AgentProfile>,
    
    #[account(mut)]
    pub poster: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimTask<'info> {
    #[account(
        mut,
        has_one = claimer @ AgentListError::NotClaimer,
    )]
    pub task: Account<'info, Task>,
    
    pub claimer: Signer<'info>,
}

#[derive(Accounts)]
pub struct SubmitCompletion<'info> {
    #[account(mut)]
    pub task: Account<'info, Task>,
    
    pub claimer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ApproveTask<'info> {
    #[account(mut)]
    pub task: Account<'info, Task>,
    
    /// CHECK: Escrow PDA
    #[account(
        mut,
        seeds = [b"escrow", task.key().as_ref()],
        bump
    )]
    pub escrow: AccountInfo<'info>,
    
    #[account(
        mut,
        seeds = [b"agent", poster.key().as_ref()],
        bump = poster_profile.bump
    )]
    pub poster_profile: Account<'info, AgentProfile>,
    
    #[account(
        mut,
        seeds = [b"agent", task.claimer.unwrap().as_ref()],
        bump = claimer_profile.bump
    )]
    pub claimer_profile: Account<'info, AgentProfile>,
    
    /// CHECK: Claimer receives payment
    #[account(mut)]
    pub claimer: AccountInfo<'info>,
    
    #[account(mut)]
    pub poster: Signer<'info>,
}

#[derive(Accounts)]
pub struct DisputeTask<'info> {
    #[account(mut)]
    pub task: Account<'info, Task>,
    
    /// CHECK: Escrow PDA
    #[account(
        mut,
        seeds = [b"escrow", task.key().as_ref()],
        bump
    )]
    pub escrow: AccountInfo<'info>,
    
    #[account(mut)]
    pub poster: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct AgentProfile {
    pub authority: Pubkey,
    #[max_len(32)]
    pub name: String,
    #[max_len(10, 32)]
    pub skills: Vec<String>,
    #[max_len(200)]
    pub bio: String,
    pub reputation: u64,
    pub tasks_completed: u64,
    pub tasks_posted: u64,
    pub total_earned: u64,
    pub total_paid: u64,
    pub registered_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Task {
    pub poster: Pubkey,
    #[max_len(500)]
    pub description: String,
    pub budget: u64,
    pub deadline: i64,
    #[max_len(5, 32)]
    pub required_skills: Vec<String>,
    pub status: TaskStatus,
    pub claimer: Option<Pubkey>,
    pub posted_at: i64,
    pub claimed_at: Option<i64>,
    pub submitted_at: Option<i64>,
    pub completed_at: Option<i64>,
    #[max_len(200)]
    pub proof_uri: Option<String>,
    #[max_len(500)]
    pub summary: Option<String>,
    #[max_len(500)]
    pub dispute_reason: Option<String>,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum TaskStatus {
    Open,
    InProgress,
    PendingApproval,
    Completed,
    Disputed,
    Cancelled,
}

#[error_code]
pub enum AgentListError {
    #[msg("Task is not open")]
    TaskNotOpen,
    #[msg("Task has expired")]
    TaskExpired,
    #[msg("Task is not in progress")]
    TaskNotInProgress,
    #[msg("Task is not pending approval")]
    TaskNotPendingApproval,
    #[msg("Not the task claimer")]
    NotClaimer,
    #[msg("Not the task poster")]
    NotPoster,
    #[msg("No claimer assigned")]
    NoClaimer,
}
