use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh1::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};

use crate::{
    error::VaultError,
    instruction::SmartVaultInstrunction,
    state::{
        VaultAppCounterState, VaultAppState, VaultBidderState, VaultMetaDataState, VaultUserState,
        VaultUserSubscriptionState,
    },
    utils::{is_ata_owner, is_valid_consesues},
};

use spl_token::{instruction::transfer, state::Account, ID as TOKEN_PROGRAM_ID};

static VAULT_METADATA: &str = "METADATA";
static APP_COUNTER: &str = "APP_COUNTER";
static APP_STATE: &str = "APP_STATE";
static USER_STATE: &str = "USER_STATE";
static SUB_STATE: &str = "SUB_STATE";
static TREASURY_STATE: &str = "TREASURY_STATE";
static BIDDER_STATE: &str = "BIDDER_STATE";

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instuction = SmartVaultInstrunction::unpack(instruction_data)?;
    match instuction {
        SmartVaultInstrunction::Init {
            vault_public_key,
            attestation_proof,
        } => init(program_id, accounts, &vault_public_key, attestation_proof),
        SmartVaultInstrunction::Join {
            attestation_proof,
            transit_key,
        } => join(&transit_key, attestation_proof),
        SmartVaultInstrunction::AddApp {
            rent_amount,
            ipfs_hash,
        } => add_app(program_id, accounts, ipfs_hash, rent_amount),
        SmartVaultInstrunction::Bid {
            _signature,
            bid_amount,
        } => bid(program_id, accounts, _signature, bid_amount),
        SmartVaultInstrunction::ClaimBid { _signature } => {
            claimbid(program_id, accounts, _signature)
        }
        SmartVaultInstrunction::CloseSub {} => close_sub(program_id, accounts),
        SmartVaultInstrunction::ReportWork { nonce, _signature } => {
            report_work(program_id, accounts, nonce, _signature)
        }
        SmartVaultInstrunction::StartSubscription {
            max_rent,
            app_id,
            params_hash,
        } => start_subscription(program_id, accounts, max_rent, app_id, params_hash),
        SmartVaultInstrunction::TopUp { amount } => topup(program_id, accounts, amount),
    }
}

pub fn init(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    vault_public_key: &Pubkey,
    attestation_proof: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;
    let app_counter = next_account_info(account_info_iter)?;
    let program_treasury = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // TODO: in future add logic of consesues rolling. Also integrate chainlink functions for attestation verification
    if !initializer.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (pda, _bump_seed) = Pubkey::find_program_address(&[VAULT_METADATA.as_bytes()], program_id);

    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let (app_counter_pda, _counter_bump_seed) =
        Pubkey::find_program_address(&[APP_COUNTER.as_bytes()], program_id);

    if app_counter_pda != *app_counter.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let (program_treasury_pda, _treasury_bump_seed) =
        Pubkey::find_program_address(&[TREASURY_STATE.as_bytes()], program_id);

    if program_treasury_pda != *program_treasury.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let state_size = 1 + 32 + (4 + attestation_proof.len());
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(state_size);

    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            pda_account.key,
            rent_lamports,
            state_size.try_into().unwrap(),
            program_id,
        ),
        &[
            initializer.clone(),
            pda_account.clone(),
            system_program.clone(),
        ],
        &[&[VAULT_METADATA.as_bytes(), &[_bump_seed]]],
    )?;

    let state_size = 8 + 1;
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(state_size);

    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            app_counter.key,
            rent_lamports,
            state_size.try_into().unwrap(),
            program_id,
        ),
        &[
            initializer.clone(),
            app_counter.clone(),
            system_program.clone(),
        ],
        &[&[APP_COUNTER.as_bytes(), &[_counter_bump_seed]]],
    )?;

    let state_size = 0;
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(state_size);

    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            program_treasury.key,
            rent_lamports,
            state_size.try_into().unwrap(),
            program_id,
        ),
        &[
            initializer.clone(),
            program_treasury.clone(),
            system_program.clone(),
        ],
        &[&[TREASURY_STATE.as_bytes(), &[_treasury_bump_seed]]],
    )?;

    let mut account_data =
        try_from_slice_unchecked::<VaultMetaDataState>(&pda_account.data.borrow()).unwrap();

    if account_data.is_initialized() {
        msg!("Protocol init already completed!");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let mut app_counter_data: VaultAppCounterState =
        try_from_slice_unchecked::<VaultAppCounterState>(&app_counter.data.borrow()).unwrap();

    if app_counter_data.is_initialized() {
        msg!("App counter acc already exsist!");
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    msg!("ProtocolInit:{}:{}", vault_public_key, attestation_proof);
    account_data.attestation_proof = attestation_proof;
    account_data.vault_public_key = *vault_public_key;
    account_data.is_initialized = true;
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;

    app_counter_data.is_initialized = true;
    app_counter_data.serialize(&mut &mut app_counter.data.borrow_mut()[..])?;

    Ok(())
}

pub fn join(transit_key: &Pubkey, attestation_proof: String) -> ProgramResult {
    // TODO: on join, server providers need to lock some amount of token and there will be state associated with their acc to maintain reputation and locked tokens
    msg!("JoinReq:{}:{}", transit_key, attestation_proof);
    Ok(())
}

pub fn add_app(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ipfs_hash: String,
    rent_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let creator = next_account_info(account_info_iter)?;
    let creator_ata = next_account_info(account_info_iter)?;
    let app_counter = next_account_info(account_info_iter)?;
    let app_state = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // TODO: if app is private/permissioned following logic will be followed. For app going to visible in public marketplace they should be included with dao or community voting approval

    if !creator.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !is_ata_owner(creator.key, creator_ata) {
        msg!("Wrong spl token account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let (app_id_counter_pda, _) =
        Pubkey::find_program_address(&[APP_COUNTER.as_bytes()], program_id);

    if app_id_counter_pda != *app_counter.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let mut app_counter_data: VaultAppCounterState =
        try_from_slice_unchecked::<VaultAppCounterState>(&app_counter.data.borrow()).unwrap();

    if !app_counter_data.is_initialized() {
        msg!("App counter not init yet");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let (app_state_pda, _app_state_bump_seed) = Pubkey::find_program_address(
        &[
            APP_STATE.as_bytes(),
            app_counter_data.counter.to_be_bytes().as_ref(),
        ],
        program_id,
    );

    if app_state_pda != *app_state.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let state_size = 1 + (4 + ipfs_hash.len()) + 8 + 32;
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(state_size);

    invoke_signed(
        &system_instruction::create_account(
            creator.key,
            &app_state.key,
            rent_lamports,
            state_size.try_into().unwrap(),
            program_id,
        ),
        &[creator.clone(), app_state.clone(), system_program.clone()],
        &[&[
            APP_STATE.as_bytes(),
            app_counter_data.counter.to_be_bytes().as_ref(),
            &[_app_state_bump_seed],
        ]],
    )?;

    let mut app_state_data =
        try_from_slice_unchecked::<VaultAppState>(&app_state.data.borrow()).unwrap();

    if app_state_data.is_initialized() {
        msg!("App already initalised");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    app_state_data.is_initialized = true;
    app_state_data.ipfs_hash = ipfs_hash;
    app_state_data.rent = rent_amount;
    app_state_data.creator_ata = *creator_ata.key;
    app_state_data.serialize(&mut &mut app_state.data.borrow_mut()[..])?;

    app_counter_data.counter += 1;
    app_counter_data.serialize(&mut &mut app_counter.data.borrow_mut()[..])?;

    Ok(())
}

pub fn topup(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user: &AccountInfo<'_> = next_account_info(account_info_iter)?;
    let user_ata: &AccountInfo<'_> = next_account_info(account_info_iter)?;
    let user_state: &AccountInfo<'_> = next_account_info(account_info_iter)?;
    let program_treasury = next_account_info(account_info_iter)?;
    let program_ata = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    if !user.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !is_ata_owner(user.key, user_ata) {
        msg!("Wrong ata provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if program_treasury.owner != program_id {
        msg!("Wrong treasury account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let (program_treasury_pda, _) =
        Pubkey::find_program_address(&[TREASURY_STATE.as_bytes()], program_id);

    if program_treasury_pda != *program_treasury.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    if !is_ata_owner(program_treasury.key, program_ata) {
        msg!("Wrong treasury ata account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if amount <= 0 {
        msg!("Amount can't be less then eq to zero");
        return Err(VaultError::LessThenMinimumTopupAmount.into());
    }

    let user_ata_data = Account::unpack(&user_ata.data.borrow())?;
    if user_ata_data.amount < amount {
        msg!("Insufficient funds to topup!");
        return Err(ProgramError::InsufficientFunds);
    }

    let (user_state_pda, _bump_seed) =
        Pubkey::find_program_address(&[USER_STATE.as_bytes(), user.key.as_ref()], program_id);

    if user_state_pda != *user_state.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    if user_state.data.borrow().len() <= 0 {
        let state_size = 1 + 8 + 8;
        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(state_size);

        invoke_signed(
            &system_instruction::create_account(
                user.key,
                &user_state.key,
                rent_lamports,
                state_size.try_into().unwrap(),
                program_id,
            ),
            &[user.clone(), user_state.clone(), system_program.clone()],
            &[&[USER_STATE.as_bytes(), user.key.as_ref(), &[_bump_seed]]],
        )?;
    }

    if user_state.owner != program_id {
        msg!("Wrong user state account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let mut user_state_data =
        try_from_slice_unchecked::<VaultUserState>(&user_state.data.borrow()).unwrap();

    let transfer_tokens_to_programm = transfer(
        &TOKEN_PROGRAM_ID,
        user_ata.key,
        program_ata.key,
        user.key,
        &[],
        amount,
    )?;

    invoke(
        &transfer_tokens_to_programm,
        &[
            user_ata.clone(),
            program_ata.clone(),
            user.clone(),
            token_program.clone(),
        ],
    )?;

    user_state_data.balance += amount;
    user_state_data.is_initialized = true;
    user_state_data.serialize(&mut &mut user_state.data.borrow_mut()[..])?;

    Ok(())
}

pub fn start_subscription(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    max_rent: u64,
    app_id: u64,
    params_hash: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let subscriber = next_account_info(account_info_iter)?;
    let subscriber_state = next_account_info(account_info_iter)?;
    let subscriber_sub_state = next_account_info(account_info_iter)?;
    let app_state: &AccountInfo<'_> = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    if !subscriber.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if app_state.owner != program_id {
        msg!("Wrong app state account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if subscriber_state.owner != program_id {
        msg!("invalid subscriber state pda");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let (app_state_pda, _) = Pubkey::find_program_address(
        &[APP_STATE.as_bytes(), app_id.to_be_bytes().as_ref()],
        program_id,
    );
    if app_state_pda != *app_state.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let app_state_data =
        try_from_slice_unchecked::<VaultAppState>(&app_state.data.borrow()).unwrap();
    if !app_state_data.is_initialized() {
        msg!("given app not found");
        return Err(ProgramError::UninitializedAccount);
    }

    let (subscriber_state_pda, _) = Pubkey::find_program_address(
        &[USER_STATE.as_bytes(), subscriber.key.as_ref()],
        program_id,
    );
    if subscriber_state_pda != *subscriber_state.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let mut subscriber_state_data =
        try_from_slice_unchecked::<VaultUserState>(&subscriber_state.data.borrow()).unwrap();

    if !subscriber_state_data.is_initialized() {
        msg!("Init/topup account first to start subscription");
        return Err(ProgramError::UninitializedAccount);
    }

    if subscriber_state_data.balance < max_rent {
        msg!("topup account to provide rent for atleast one cycle");
        return Err(VaultError::InefficientBalance.into());
    }

    let (subscriber_sub_state_pda, _bump_seed) = Pubkey::find_program_address(
        &[
            SUB_STATE.as_bytes(),
            subscriber.key.as_ref(),
            subscriber_state_data.count.to_be_bytes().as_ref(),
        ],
        program_id,
    );

    if subscriber_sub_state_pda != *subscriber_sub_state.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let state_size = 8 + 1 + 1 + 8 + (4 + params_hash.len()) + 8 + 1 + 32 + 8 + 8 + 8 + 8 + 1;
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(state_size);

    invoke_signed(
        &system_instruction::create_account(
            subscriber.key,
            &subscriber_sub_state.key,
            rent_lamports,
            state_size.try_into().unwrap(),
            program_id,
        ),
        &[
            subscriber.clone(),
            subscriber_sub_state.clone(),
            system_program.clone(),
        ],
        &[&[
            SUB_STATE.as_bytes(),
            subscriber.key.as_ref(),
            subscriber_state_data.count.to_be_bytes().as_ref(),
            &[_bump_seed],
        ]],
    )?;

    let mut subscriber_sub_state_data =
        try_from_slice_unchecked::<VaultUserSubscriptionState>(&subscriber_sub_state.data.borrow())
            .unwrap();

    if subscriber_sub_state_data.is_initialized() {
        msg!("sub state already initalised");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let clock = Clock::get()?;
    subscriber_sub_state_data.id = subscriber_state_data.count;
    subscriber_sub_state_data.app_id = app_id;
    subscriber_sub_state_data.is_initialized = true;
    subscriber_sub_state_data.params_hash = params_hash;
    subscriber_sub_state_data.max_rent = max_rent;
    subscriber_sub_state_data.rent = max_rent;
    subscriber_sub_state_data.bid_endtime = clock.unix_timestamp as u64 + 60;

    subscriber_sub_state_data.serialize(&mut &mut subscriber_sub_state.data.borrow_mut()[..])?;

    subscriber_state_data.count += 1;
    subscriber_state_data.serialize(&mut &mut subscriber_state.data.borrow_mut()[..])?;

    msg!(
        "SubRequest:{}:{}:{}:{}",
        subscriber.key,
        subscriber_state_data.count - 1,
        max_rent,
        subscriber_sub_state_data.bid_endtime
    );

    Ok(())
}

pub fn bid(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _signature: [u8; 64],
    bid_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let consensus = next_account_info(account_info_iter)?;
    let bidder = next_account_info(account_info_iter)?;
    let bidder_state = next_account_info(account_info_iter)?;
    let sub_state: &AccountInfo<'_> = next_account_info(account_info_iter)?;
    let metadata: &AccountInfo<'_> = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let ix_sysvar: &AccountInfo<'_> = next_account_info(account_info_iter)?;

    if !bidder.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if sub_state.owner != program_id {
        msg!("Wrong sub state account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let (bidder_state_pda, _bump_seed) =
        Pubkey::find_program_address(&[BIDDER_STATE.as_bytes(), bidder.key.as_ref()], program_id);

    if bidder_state_pda != *bidder_state.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    if bidder_state.data.borrow().len() <= 0 {
        let state_size = 1 + 8;
        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(state_size);

        invoke_signed(
            &system_instruction::create_account(
                bidder.key,
                &bidder_state.key,
                rent_lamports,
                state_size.try_into().unwrap(),
                program_id,
            ),
            &[bidder.clone(), bidder_state.clone(), system_program.clone()],
            &[&[BIDDER_STATE.as_bytes(), bidder.key.as_ref(), &[_bump_seed]]],
        )?;
    }

    if bidder_state.owner != program_id {
        msg!("Wrong bidder state account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let mut bidder_state_data =
        try_from_slice_unchecked::<VaultBidderState>(&bidder_state.data.borrow()).unwrap();

    let mut raw_message: [u8; 40] = [0; 40];
    raw_message[..32].copy_from_slice(bidder.key.to_bytes().as_ref());
    raw_message[32..].copy_from_slice(bidder_state_data.nonce.to_be_bytes().as_ref());

    is_valid_consesues(
        VAULT_METADATA,
        ix_sysvar,
        consensus,
        metadata,
        program_id,
        raw_message.as_ref(),
        _signature.as_ref(),
    )?;

    let mut sub_state_data =
        try_from_slice_unchecked::<VaultUserSubscriptionState>(&sub_state.data.borrow()).unwrap();

    if !sub_state_data.is_initialized() {
        msg!("subsciption not init yet!");
        return Err(ProgramError::UninitializedAccount);
    }

    let clock = Clock::get()?;
    let cur_time = clock.unix_timestamp as u64;

    // TODO: Currently least rent amount bidder wins. But in future apart from least rent winner should be also selected by keeping reputation factor in mind.
    if cur_time < sub_state_data.bid_endtime {
        if bid_amount < sub_state_data.rent {
            sub_state_data.executor = *bidder.key;
            sub_state_data.rent = bid_amount;
        } else if bid_amount == sub_state_data.rent {
            if sub_state_data.executor == *system_program.key {
                sub_state_data.executor = *bidder.key;
            }
        }
    } else {
        msg!("Bid time expired");
        return Err(VaultError::BidTimeExpired.into());
    }

    bidder_state_data.is_initialized = true;
    bidder_state_data.nonce += 1;
    bidder_state_data.serialize(&mut &mut bidder_state.data.borrow_mut()[..])?;

    sub_state_data.serialize(&mut &mut sub_state.data.borrow_mut()[..])?;

    msg!("BidAdded:{}:{}", sub_state.key, sub_state_data.rent);
    Ok(())
}

pub fn claimbid(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _signature: [u8; 64],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let consensus: &AccountInfo<'_> = next_account_info(account_info_iter)?;
    let bid_winner = next_account_info(account_info_iter)?;
    let bid_winner_state = next_account_info(account_info_iter)?;
    let sub_state: &AccountInfo<'_> = next_account_info(account_info_iter)?;
    let metadata: &AccountInfo<'_> = next_account_info(account_info_iter)?;
    let ix_sysvar: &AccountInfo<'_> = next_account_info(account_info_iter)?;

    if !bid_winner.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if bid_winner_state.owner != program_id {
        msg!("Wrong bidder state account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if sub_state.owner != program_id {
        msg!("Wrong sub state account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let (bidder_state_pda, _) = Pubkey::find_program_address(
        &[BIDDER_STATE.as_bytes(), bid_winner.key.as_ref()],
        program_id,
    );

    if bidder_state_pda != *bid_winner_state.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let mut bid_winner_state_data =
        try_from_slice_unchecked::<VaultBidderState>(&bid_winner_state.data.borrow()).unwrap();

    if !bid_winner_state_data.is_initialized() {
        msg!("Invalid bidder state details provided!");
        return Err(ProgramError::UninitializedAccount);
    }

    let mut raw_message: [u8; 40] = [0; 40];
    raw_message[..32].copy_from_slice(bid_winner.key.to_bytes().as_ref());
    raw_message[32..].copy_from_slice(bid_winner_state_data.nonce.to_be_bytes().as_ref());

    is_valid_consesues(
        VAULT_METADATA,
        ix_sysvar,
        consensus,
        metadata,
        program_id,
        raw_message.as_ref(),
        _signature.as_ref(),
    )?;

    let mut sub_state_data =
        try_from_slice_unchecked::<VaultUserSubscriptionState>(&sub_state.data.borrow()).unwrap();

    if !sub_state_data.is_initialized() {
        msg!("Invalid subscription details provided!");
        return Err(ProgramError::UninitializedAccount);
    }

    if sub_state_data.closed {
        msg!("subscription already closed!");
        return Err(VaultError::SubScriptionClosed.into());
    }

    if sub_state_data.is_assigned {
        msg!("Bid already claimed!");
        return Err(VaultError::BidAlreadyClaimed.into());
    }

    if sub_state_data.executor != *bid_winner.key {
        msg!("You do not won the bid!");
        return Err(VaultError::UnAuthToClaimBid.into());
    }

    let clock = Clock::get()?;
    let cur_time = clock.unix_timestamp as u64;
    if cur_time > sub_state_data.bid_endtime + 300 {
        msg!("You failed to claim bid!");
        //TODO: add logic to decrease the reputation of bid winner
        return Err(VaultError::BidClaimExpired.into());
    } else if cur_time < sub_state_data.bid_endtime {
        msg!("Trying to claim bid too early!");
        return  Err(VaultError::ReportedEarly.into());
    }

    bid_winner_state_data.nonce += 1;
    bid_winner_state_data.serialize(&mut &mut bid_winner_state.data.borrow_mut()[..])?;

    sub_state_data.is_assigned = true;
    sub_state_data.last_report_time = cur_time;
    sub_state_data.serialize(&mut &mut sub_state.data.borrow_mut()[..])?;

    Ok(())
}

pub fn report_work(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    nonce: u64,
    _signature: [u8; 64],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let consensus = next_account_info(account_info_iter)?;
    let bid_winner = next_account_info(account_info_iter)?;
    let bid_winner_state = next_account_info(account_info_iter)?;
    let bid_winner_ata = next_account_info(account_info_iter)?;
    let sub_state = next_account_info(account_info_iter)?;
    let user = next_account_info(account_info_iter)?;
    let user_state = next_account_info(account_info_iter)?;
    let program_treasury = next_account_info(account_info_iter)?;
    let program_ata = next_account_info(account_info_iter)?;
    let metadata = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let ix_sysvar = next_account_info(account_info_iter)?;

    if !bid_winner.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if bid_winner_state.owner != program_id {
        msg!("Wrong bidder state account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if sub_state.owner != program_id {
        msg!("Wrong sub state account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if user_state.owner != program_id {
        msg!("Wrong app state account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !is_ata_owner(bid_winner.key, bid_winner_ata) {
        msg!("Wrong associated token account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if program_treasury.owner != program_id {
        msg!("Wrong programme ata account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let (program_treasury_pda, _pata_bump_seed) =
        Pubkey::find_program_address(&[TREASURY_STATE.as_bytes()], program_id);

    if program_treasury_pda != *program_treasury.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    if !is_ata_owner(program_treasury.key, program_ata) {
        msg!("Wrong treasury ata account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let (bidder_state_pda, _) = Pubkey::find_program_address(
        &[BIDDER_STATE.as_bytes(), bid_winner.key.as_ref()],
        program_id,
    );

    if bidder_state_pda != *bid_winner_state.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let (user_state_pda, _) =
        Pubkey::find_program_address(&[USER_STATE.as_bytes(), user.key.as_ref()], program_id);

    if user_state_pda != *user_state.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    let mut user_state_data =
        try_from_slice_unchecked::<VaultUserState>(&user_state.data.borrow()).unwrap();

    if !user_state_data.is_initialized() {
        msg!("User not found!");
        return Err(ProgramError::UninitializedAccount);
    }

    let mut bid_winner_state_data =
        try_from_slice_unchecked::<VaultBidderState>(&bid_winner_state.data.borrow()).unwrap();

    if !bid_winner_state_data.is_initialized() {
        msg!("Invalid bidder state details provided!");
        return Err(ProgramError::UninitializedAccount);
    }

    let mut raw_message: [u8; 40] = [0; 40];
    raw_message[..32].copy_from_slice(bid_winner.key.to_bytes().as_ref());
    raw_message[32..].copy_from_slice(bid_winner_state_data.nonce.to_be_bytes().as_ref());

    is_valid_consesues(
        VAULT_METADATA,
        ix_sysvar,
        consensus,
        metadata,
        program_id,
        raw_message.as_ref(),
        _signature.as_ref(),
    )?;

    let mut sub_state_data =
        try_from_slice_unchecked::<VaultUserSubscriptionState>(&sub_state.data.borrow()).unwrap();

    if !sub_state_data.is_initialized() {
        msg!("Invalid subscription details provided!");
        return Err(ProgramError::UninitializedAccount);
    }

    let (sub_state_pda, _) = Pubkey::find_program_address(
        &[
            SUB_STATE.as_bytes(),
            user.key.as_ref(),
            sub_state_data.id.to_be_bytes().as_ref(),
        ],
        program_id,
    );

    if sub_state_pda != *sub_state.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    if sub_state_data.closed {
        msg!("subscription already closed!");
        return Err(VaultError::SubScriptionClosed.into());
    }

    if !sub_state_data.is_assigned {
        msg!("Bid not assigned yet!");
        return Err(VaultError::BidAlreadyClaimed.into());
    }

    if sub_state_data.executor != *bid_winner.key {
        msg!("You do not have permission to maintain the subscription!");
        return Err(VaultError::UnAuthToClaimBid.into());
    }

    if sub_state_data.restart {
        msg!("Its currently on re-assign mode and rewards not claimable!");
        return Err(VaultError::RestartPhase.into());
    }

    let clock = Clock::get()?;
    let cur_time = clock.unix_timestamp as u64;
    if cur_time > sub_state_data.last_report_time + 900 {
        msg!("worker/bid winner failed to provide SLA!");
        // TODO: add logic to decrease reputation
        // use this restart flag in future where bots report the worker and help protocol to restart the particular subscription/ work and get some reword for this good work by slashing it from the locked dpeosits of workers
        sub_state_data.restart = true;
    } else if cur_time < sub_state_data.last_report_time + 600 {
        msg!("reported too early!");
        return Err(VaultError::ReportedEarly.into());
    }

    // mechanism to check worker working correctly
    if sub_state_data.nonce != nonce {
        sub_state_data.restart = true;
    }

    if !user_state_data.balance < sub_state_data.rent {
        sub_state_data.closed = true;
        msg!("Insufficient balance");
        msg!("SubClosed:{}", sub_state.key);
    } else if !sub_state_data.restart {
        let transfer_token_to_worker = transfer(
            &TOKEN_PROGRAM_ID,
            program_ata.key,
            bid_winner_ata.key,
            program_treasury.key,
            &[],
            sub_state_data.rent,
        )?;

        invoke_signed(
            &transfer_token_to_worker,
            &[
                program_ata.clone(),
                bid_winner_ata.clone(),
                program_treasury.clone(),
                token_program.clone(),
            ],
            &[&[TREASURY_STATE.as_bytes(), &[_pata_bump_seed]]],
        )?;

        sub_state_data.nonce += 1;
        sub_state_data.last_report_time = cur_time;
        user_state_data.balance -= sub_state_data.rent;
        user_state_data.serialize(&mut &mut user_state.data.borrow_mut()[..])?;
    }

    bid_winner_state_data.nonce += 1;
    bid_winner_state_data.serialize(&mut &mut bid_winner_state.data.borrow_mut()[..])?;

    sub_state_data.serialize(&mut &mut sub_state.data.borrow_mut()[..])?;
    Ok(())
}

pub fn close_sub(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user: &AccountInfo<'_> = next_account_info(account_info_iter)?;
    let user_sub: &AccountInfo<'_> = next_account_info(account_info_iter)?;

    if !user.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if user_sub.owner != program_id {
        msg!("Wrong sub state account provided");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let mut sub_state_data =
        try_from_slice_unchecked::<VaultUserSubscriptionState>(&user_sub.data.borrow()).unwrap();

    if !sub_state_data.is_initialized() {
        msg!("Invalid user sub state details provided!");
        return Err(ProgramError::UninitializedAccount);
    }

    let (user_sub_pda, _) = Pubkey::find_program_address(
        &[
            SUB_STATE.as_bytes(),
            user.key.as_ref(),
            sub_state_data.id.to_be_bytes().as_ref(),
        ],
        program_id,
    );

    if user_sub_pda != *user_sub.key {
        msg!("Invalid seeds for PDA");
        return Err(VaultError::InvalidPDA.into());
    }

    sub_state_data.closed = true;
    sub_state_data.serialize(&mut &mut user_sub.data.borrow_mut()[..])?;
    msg!("SubClosed:{}", user_sub.key);

    Ok(())
}

// TODO: add function by which bots can help to re-assign restarted subscription. And reset their state like nonce, executor and etc. For this they will also get reward which will be slashed from bad worker's locked deposit
