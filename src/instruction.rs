use borsh::BorshDeserialize;
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

pub enum SmartVaultInstrunction {
    Init {
        vault_public_key: Pubkey,
        attestation_proof: String,
    },
    Join {
        attestation_proof: String,
        transit_key: Pubkey,
        p2p_connection: String,
    },
    AddApp {
        rent_amount: u64,
        ipfs_hash: String,
    },
    TopUp {
        amount: u64,
    },
    StartSubscription {
        max_rent: u64,
        app_id: u64,
        params_hash: String,
    },
    Bid {
        _signature: String,
        bid_amount: u64,
    },
    ClaimBid {
        _signature: String,
    },
    ReportWork {
        nonce: u64,
        _signature: String,
    },
    CloseSub {},
}

#[derive(Debug, BorshDeserialize)]
struct InitPayload {
    vault_public_key: Pubkey,
    attestation_proof: String,
}

#[derive(BorshDeserialize)]
struct JoinPayload {
    attestation_proof: String,
    transit_key: Pubkey,
    p2p_connection: String,
}

#[derive(BorshDeserialize)]
struct AddAppPayload {
    rent_amount: u64,
    ipfs_hash: String,
}

#[derive(BorshDeserialize)]
struct TopUpPayload {
    amount: u64,
}

#[derive(BorshDeserialize)]
struct StartSubscriptionPayload {
    max_rent: u64,
    app_id: u64,
    params_hash: String,
}

#[derive(BorshDeserialize)]
struct BidPayload {
    _signature: String,
    bid_amount: u64,
}

#[derive(BorshDeserialize)]
struct ClaimBidPayload {
    _signature: String,
}

#[derive(BorshDeserialize)]
struct ReportWorkPayload {
    nonce: u64,
    _signature: String,
}

impl SmartVaultInstrunction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        Ok(match variant {
            0 => {
                let payload = InitPayload::try_from_slice(rest).unwrap();
                Self::Init {
                    vault_public_key: payload.vault_public_key,
                    attestation_proof: payload.attestation_proof,
                }
            }
            1 => {
                let payload = JoinPayload::try_from_slice(rest).unwrap();
                Self::Join {
                    attestation_proof: payload.attestation_proof,
                    transit_key: payload.transit_key,
                    p2p_connection: payload.p2p_connection,
                }
            }
            2 => {
                let payload = AddAppPayload::try_from_slice(rest).unwrap();
                Self::AddApp {
                    rent_amount: payload.rent_amount,
                    ipfs_hash: payload.ipfs_hash,
                }
            }
            3 => {
                let payload = TopUpPayload::try_from_slice(rest).unwrap();
                Self::TopUp {
                    amount: payload.amount,
                }
            }
            4 => {
                let payload = StartSubscriptionPayload::try_from_slice(rest).unwrap();
                Self::StartSubscription {
                    max_rent: payload.max_rent,
                    app_id: payload.app_id,
                    params_hash: payload.params_hash,
                }
            }
            5 => {
                let payload = BidPayload::try_from_slice(rest).unwrap();
                Self::Bid {
                    _signature: payload._signature,
                    bid_amount: payload.bid_amount,
                }
            }
            6 => {
                let payload = ClaimBidPayload::try_from_slice(rest).unwrap();
                Self::ClaimBid {
                    _signature: payload._signature,
                }
            }
            7 => {
                let payload = ReportWorkPayload::try_from_slice(rest).unwrap();
                Self::ReportWork {
                    nonce: payload.nonce,
                    _signature: payload._signature,
                }
            }
            8 => Self::CloseSub {},
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}
