use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_pack::IsInitialized,
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct VaultMetaDataState {
    pub is_initialized: bool,
    pub attestation_proof: String,
    pub vault_public_key: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct VaultAppCounterState {
    pub is_initialized: bool,
    pub counter: u64,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct VaultAppState {
    pub is_initialized: bool,
    pub ipfs_hash: String,
    pub rent: u64,
    pub creator_ata: Pubkey
}



#[derive(BorshSerialize, BorshDeserialize)]
pub struct VaultUserState {
    pub is_initialized: bool,
    pub count: u64,
    pub balance: u64
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct VaultUserSubscriptionState {
    pub id: u64,
    pub is_initialized: bool,
    pub closed: bool,
    pub app_id: u64,
    pub params_hash: String,
    pub max_rent: u64,
    pub is_assigned: bool,
    pub executor: Pubkey,
    pub bid_endtime: u64,
    pub rent: u64,
    pub nonce: u64,
    pub last_report_time: u64,
    pub restart: bool
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct VaultBidderState {
    pub is_initialized: bool,
    pub nonce: u64
}

impl IsInitialized for VaultMetaDataState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for VaultAppCounterState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for VaultAppState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for VaultUserState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for VaultUserSubscriptionState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}


impl IsInitialized for VaultBidderState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

