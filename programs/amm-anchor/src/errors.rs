use anchor_lang::prelude::*;
use constant_product_curve::CurveError;

#[error_code]
pub enum AmmError {
    #[msg("The AMM pool is currently locked.")]
        AmmLocked,
    #[msg("Invalid amount provided.")]
    InvalidAmount,
    #[msg("Slippage exceeded.")]
    SlippageExceeded,
    #[msg("Insufficient balance for the operation.")]
    Insufficientbalance,
    #[msg("Overflow occurred during calculation.")]
    Overflow,
    #[msg("Underflow occurred during calculation.")]
    Underflow,
    #[msg("Invalid precision specified.")]
    InvalidPrecision,
    #[msg("Invalid fee amount specified.")]
    InvalidFee,
    #[msg("Zero balance encountered.")]
    ZeroBalance,
}

impl From<CurveError> for AmmError {
    fn from(error: CurveError) -> AmmError {
        match error {
            CurveError::InvalidPrecision => AmmError::InvalidPrecision,
            CurveError::Overflow => AmmError::Overflow,
            CurveError::Underflow => AmmError::Underflow,
            CurveError::InvalidFeeAmount => AmmError::InvalidFee,
            CurveError::InsufficientBalance => AmmError::Insufficientbalance,
            CurveError::ZeroBalance => AmmError::ZeroBalance,
            CurveError::SlippageLimitExceeded => AmmError::SlippageExceeded,
        }
    }
}