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
