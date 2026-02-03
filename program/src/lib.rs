use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = AgentListInstruction::try_from_slice(instruction_data)?;

    match instruction {
        AgentListInstruction::RegisterAgent { name, skills, bio } => {
            register_agent(program_id, accounts, name, skills, bio)
        }
        AgentListInstruction::PostTask {
            description,
            budget,
            deadline,
            required_skills,
        } => post_task(program_id, accounts, description, budget, deadline, required_skills),
        AgentListInstruction::ClaimTask => claim_task(program_id, accounts),
        AgentListInstruction::SubmitCompletion { proof_uri, summary } => {
            submit_completion(program_id, accounts, proof_uri, summary)
        }
        AgentListInstruction::ApproveTask => approve_task(program_id, accounts),
        AgentListInstruction::DisputeTask { reason } => dispute_task(program_id, accounts, reason),
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum AgentListInstruction {
    RegisterAgent {
        name: String,
        skills: Vec<String>,
        bio: String,
    },
    PostTask {
        description: String,
        budget: u64,
        deadline: i64,
        required_skills: Vec<String>,
    },
    ClaimTask,
    SubmitCompletion {
        proof_uri: String,
        summary: String,
    },
    ApproveTask,
    DisputeTask {
        reason: String,
    },
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct AgentProfile {
    pub authority: Pubkey,
    pub name: String,
    pub skills: Vec<String>,
    pub bio: String,
    pub reputation: u64,
    pub tasks_completed: u64,
    pub tasks_posted: u64,
    pub total_earned: u64,
    pub total_paid: u64,
    pub registered_at: i64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub enum TaskStatus {
    Open,
    InProgress,
    PendingApproval,
    Completed,
    Disputed,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Task {
    pub poster: Pubkey,
    pub description: String,
    pub budget: u64,
    pub deadline: i64,
    pub required_skills: Vec<String>,
    pub status: TaskStatus,
    pub claimer: Option<Pubkey>,
    pub posted_at: i64,
    pub claimed_at: Option<i64>,
    pub submitted_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub proof_uri: Option<String>,
    pub summary: Option<String>,
    pub dispute_reason: Option<String>,
}

fn register_agent(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    skills: Vec<String>,
    bio: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let agent_profile_account = next_account_info(account_info_iter)?;
    let authority_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Derive PDA
    let (agent_pda, bump) = Pubkey::find_program_address(
        &[b"agent", authority_account.key.as_ref()],
        program_id,
    );

    if agent_pda != *agent_profile_account.key {
        msg!("Invalid agent profile PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    let profile = AgentProfile {
        authority: *authority_account.key,
        name,
        skills,
        bio,
        reputation: 0,
        tasks_completed: 0,
        tasks_posted: 0,
        total_earned: 0,
        total_paid: 0,
        registered_at: solana_program::clock::Clock::get()?.unix_timestamp,
    };

    let data = profile.try_to_vec()?;
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(data.len());

    // Create account
    invoke(
        &system_instruction::create_account(
            authority_account.key,
            agent_profile_account.key,
            lamports,
            data.len() as u64,
            program_id,
        ),
        &[
            authority_account.clone(),
            agent_profile_account.clone(),
            system_program.clone(),
        ],
    )?;

    // Write data
    agent_profile_account.try_borrow_mut_data()?[..data.len()].copy_from_slice(&data);

    msg!("Agent registered: {}", profile.name);
    Ok(())
}

fn post_task(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    description: String,
    budget: u64,
    deadline: i64,
    required_skills: Vec<String>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let task_account = next_account_info(account_info_iter)?;
    let escrow_account = next_account_info(account_info_iter)?;
    let poster_profile_account = next_account_info(account_info_iter)?;
    let poster_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    if !poster_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load poster profile
    let mut poster_profile =
        AgentProfile::try_from_slice(&poster_profile_account.try_borrow_data()?)?;

    // Create task
    let task = Task {
        poster: *poster_account.key,
        description,
        budget,
        deadline,
        required_skills,
        status: TaskStatus::Open,
        claimer: None,
        posted_at: solana_program::clock::Clock::get()?.unix_timestamp,
        claimed_at: None,
        submitted_at: None,
        completed_at: None,
        proof_uri: None,
        summary: None,
        dispute_reason: None,
    };

    let data = task.try_to_vec()?;
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(data.len());

    // Transfer SOL to escrow
    invoke(
        &system_instruction::transfer(poster_account.key, escrow_account.key, budget),
        &[poster_account.clone(), escrow_account.clone()],
    )?;

    // Create task account
    invoke(
        &system_instruction::create_account(
            poster_account.key,
            task_account.key,
            lamports,
            data.len() as u64,
            program_id,
        ),
        &[
            poster_account.clone(),
            task_account.clone(),
            system_program.clone(),
        ],
    )?;

    // Write task data
    task_account.try_borrow_mut_data()?[..data.len()].copy_from_slice(&data);

    // Update poster profile
    poster_profile.tasks_posted += 1;
    let profile_data = poster_profile.try_to_vec()?;
    poster_profile_account.try_borrow_mut_data()?[..profile_data.len()]
        .copy_from_slice(&profile_data);

    msg!("Task posted with budget: {} lamports", budget);
    Ok(())
}

fn claim_task(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let task_account = next_account_info(account_info_iter)?;
    let claimer_account = next_account_info(account_info_iter)?;

    if !claimer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut task = Task::try_from_slice(&task_account.try_borrow_data()?)?;

    if task.status != TaskStatus::Open {
        msg!("Task is not open");
        return Err(ProgramError::InvalidAccountData);
    }

    let now = solana_program::clock::Clock::get()?.unix_timestamp;
    if now >= task.deadline {
        msg!("Task has expired");
        return Err(ProgramError::InvalidAccountData);
    }

    task.status = TaskStatus::InProgress;
    task.claimer = Some(*claimer_account.key);
    task.claimed_at = Some(now);

    let data = task.try_to_vec()?;
    task_account.try_borrow_mut_data()?[..data.len()].copy_from_slice(&data);

    msg!("Task claimed by: {}", claimer_account.key);
    Ok(())
}

fn submit_completion(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof_uri: String,
    summary: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let task_account = next_account_info(account_info_iter)?;
    let claimer_account = next_account_info(account_info_iter)?;

    if !claimer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut task = Task::try_from_slice(&task_account.try_borrow_data()?)?;

    if task.status != TaskStatus::InProgress {
        msg!("Task is not in progress");
        return Err(ProgramError::InvalidAccountData);
    }

    if task.claimer != Some(*claimer_account.key) {
        msg!("Not the task claimer");
        return Err(ProgramError::InvalidAccountData);
    }

    task.status = TaskStatus::PendingApproval;
    task.proof_uri = Some(proof_uri);
    task.summary = Some(summary);
    task.submitted_at = Some(solana_program::clock::Clock::get()?.unix_timestamp);

    let data = task.try_to_vec()?;
    task_account.try_borrow_mut_data()?[..data.len()].copy_from_slice(&data);

    msg!("Task completion submitted");
    Ok(())
}

fn approve_task(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let task_account = next_account_info(account_info_iter)?;
    let escrow_account = next_account_info(account_info_iter)?;
    let poster_profile_account = next_account_info(account_info_iter)?;
    let claimer_profile_account = next_account_info(account_info_iter)?;
    let claimer_account = next_account_info(account_info_iter)?;
    let poster_account = next_account_info(account_info_iter)?;

    if !poster_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut task = Task::try_from_slice(&task_account.try_borrow_data()?)?;

    if task.status != TaskStatus::PendingApproval {
        msg!("Task is not pending approval");
        return Err(ProgramError::InvalidAccountData);
    }

    if task.poster != *poster_account.key {
        msg!("Not the task poster");
        return Err(ProgramError::InvalidAccountData);
    }

    let claimer_key = task.claimer.ok_or(ProgramError::InvalidAccountData)?;

    // Transfer from escrow to claimer
    **escrow_account.try_borrow_mut_lamports()? -= task.budget;
    **claimer_account.try_borrow_mut_lamports()? += task.budget;

    // Update task
    task.status = TaskStatus::Completed;
    task.completed_at = Some(solana_program::clock::Clock::get()?.unix_timestamp);
    let task_data = task.try_to_vec()?;
    task_account.try_borrow_mut_data()?[..task_data.len()].copy_from_slice(&task_data);

    // Update claimer profile
    let mut claimer_profile =
        AgentProfile::try_from_slice(&claimer_profile_account.try_borrow_data()?)?;
    claimer_profile.tasks_completed += 1;
    claimer_profile.total_earned += task.budget;
    claimer_profile.reputation += 10; // Simple reputation for v1
    let claimer_data = claimer_profile.try_to_vec()?;
    claimer_profile_account.try_borrow_mut_data()?[..claimer_data.len()]
        .copy_from_slice(&claimer_data);

    // Update poster profile
    let mut poster_profile =
        AgentProfile::try_from_slice(&poster_profile_account.try_borrow_data()?)?;
    poster_profile.total_paid += task.budget;
    let poster_data = poster_profile.try_to_vec()?;
    poster_profile_account.try_borrow_mut_data()?[..poster_data.len()]
        .copy_from_slice(&poster_data);

    msg!("Task approved and payment released: {} lamports", task.budget);
    Ok(())
}

fn dispute_task(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    reason: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let task_account = next_account_info(account_info_iter)?;
    let escrow_account = next_account_info(account_info_iter)?;
    let poster_account = next_account_info(account_info_iter)?;

    if !poster_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut task = Task::try_from_slice(&task_account.try_borrow_data()?)?;

    if task.status != TaskStatus::PendingApproval {
        msg!("Task is not pending approval");
        return Err(ProgramError::InvalidAccountData);
    }

    if task.poster != *poster_account.key {
        msg!("Not the task poster");
        return Err(ProgramError::InvalidAccountData);
    }

    // Refund poster
    **escrow_account.try_borrow_mut_lamports()? -= task.budget;
    **poster_account.try_borrow_mut_lamports()? += task.budget;

    // Update task
    task.status = TaskStatus::Disputed;
    task.dispute_reason = Some(reason);
    let data = task.try_to_vec()?;
    task_account.try_borrow_mut_data()?[..data.len()].copy_from_slice(&data);

    msg!("Task disputed and payment refunded");
    Ok(())
}
