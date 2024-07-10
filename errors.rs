use ink::{
    env::Error as InkEnvError,
    prelude::{format, string::String},
    LangError,
};
use openbrush::contracts::psp22::PSP22Error;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum AzTradingCompetitionError {
    ContractCall(LangError),
    InkEnvError(String),
    NotFound(String),
    PSP22Error(PSP22Error),
    RouterError(RouterError),
    Unauthorised,
    UnprocessableEntity(String),
}
impl From<InkEnvError> for AzTradingCompetitionError {
    fn from(e: InkEnvError) -> Self {
        AzTradingCompetitionError::InkEnvError(format!("{e:?}"))
    }
}
impl From<LangError> for AzTradingCompetitionError {
    fn from(e: LangError) -> Self {
        AzTradingCompetitionError::ContractCall(e)
    }
}
impl From<PSP22Error> for AzTradingCompetitionError {
    fn from(e: PSP22Error) -> Self {
        AzTradingCompetitionError::PSP22Error(e)
    }
}
impl From<RouterError> for AzTradingCompetitionError {
    fn from(e: RouterError) -> Self {
        AzTradingCompetitionError::RouterError(e)
    }
}

// === COMMON AMM ROUTER ===
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RouterError {
    PSP22Error(PSP22Error),
    FactoryError(FactoryError),
    PairError(PairError),
    LangError(LangError),
    MathError(MathError),

    CrossContractCallFailed(String),
    Expired,
    IdenticalAddresses,
    InvalidPath,
    PairNotFound,
    TransferError,

    ExcessiveInputAmount,
    InsufficientAmount,
    InsufficientOutputAmount,
    InsufficientAmountA,
    InsufficientAmountB,
    InsufficientLiquidity,
}
macro_rules! impl_froms {
    ( $( $error:ident ),* ) => {
        $(
            impl From<$error> for RouterError {
                fn from(error: $error) -> Self {
                    RouterError::$error(error)
                }
            }
        )*
    };
}
impl_froms!(PSP22Error, FactoryError, PairError, LangError, MathError);

/// Errors that can be returned from calling `Factory`'s methods.
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum FactoryError {
    PairError(PairError),
    CallerIsNotFeeSetter,
    IdenticalAddresses,
    PairExists,
    PairInstantiationFailed,
}
impl From<PairError> for FactoryError {
    fn from(error: PairError) -> Self {
        FactoryError::PairError(error)
    }
}

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PairError {
    PSP22Error(PSP22Error),
    LangError(LangError),
    MathError(MathError),
    KInvariantChanged,
    InsufficientLiquidityMinted,
    InsufficientLiquidityBurned,
    InsufficientOutputAmount,
    InsufficientLiquidity,
    InsufficientInputAmount,
    InvalidTo,
    ReservesOverflow,
}
impl From<PSP22Error> for PairError {
    fn from(error: PSP22Error) -> Self {
        PairError::PSP22Error(error)
    }
}
impl From<LangError> for PairError {
    fn from(error: LangError) -> Self {
        PairError::LangError(error)
    }
}
impl From<MathError> for PairError {
    fn from(error: MathError) -> Self {
        PairError::MathError(error)
    }
}

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum MathError {
    AddOverflow(u8),
    CastOverflow(u8),
    DivByZero(u8),
    MulOverflow(u8),
    SubUnderflow(u8),
}
