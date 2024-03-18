use borsh::BorshDeserialize;
use solana_program::{msg, program_error::ProgramError, pubkey::Pubkey};


pub enum  SmartVaultInstrunction {
    Init {
        vault_public_key: Pubkey,
        attestation_proof: String,
    },
    Join {
        attestation_proof: String,
        transit_key: Pubkey,
        p2p_connection: String
    }
}

#[derive(Debug)]
#[derive(BorshDeserialize)]
struct InitPayload {
    vault_public_key: Pubkey,
    attestation_proof: String,
}


#[derive(BorshDeserialize)]
struct JoinPayload {
    attestation_proof: String,
    transit_key: Pubkey,
    p2p_connection: String
}

impl  SmartVaultInstrunction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError>  {
        let (&variant, rest) = input
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;
        Ok(match variant{
            0 => {
                msg!("Hello!");
                msg!("RestData: {:?}", rest);
                let payload = InitPayload::try_from_slice(rest).unwrap();
                msg!("Issue passed!");
                Self::Init { vault_public_key: payload.vault_public_key, attestation_proof:  payload.attestation_proof }
            }
            1 => {
                let payload = JoinPayload::try_from_slice(rest).unwrap();
                Self::Join { attestation_proof: payload.attestation_proof, transit_key: payload.transit_key, p2p_connection: payload.p2p_connection }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}