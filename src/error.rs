use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("PDA derived does not equal PDA passed in")]
    InvalidPDA,
    #[error("Balance is not efficient ")]
    InefficientBalance,
    #[error("Topup amount must be greater then zero")]
    LessThenMinimumTopupAmount,
    #[error("Bidding time already finished")]
    BidTimeExpired,
    #[error("Bid already claimed")]
    BidAlreadyClaimed,
    #[error("You are un-authorised to claim bid")]
    UnAuthToClaimBid,
    #[error("Bid claim time limit over")]
    BidClaimExpired,
    #[error("Given subscription is closed")]
    SubScriptionClosed,
    #[error("Report is not valid")]
    InvalidReport,
    #[error("Reported too early")]
    ReportedEarly,
    #[error("Restart phase")]
    RestartPhase,
    #[error("Valid consesues not provided")]
    InvalidConsesues,
    #[error("Signature verification failed.")]
    SigVerificationFailed,
}


impl From<VaultError> for ProgramError {
    fn from(e: VaultError) -> Self {
        ProgramError::Custom(e as u32)
    }
}