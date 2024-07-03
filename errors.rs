use ink::{
    env::Error as InkEnvError,
    prelude::{format, string::String},
    LangError,
};
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum AzTradingCompetitionError {
    ContractCall(LangError),
    InkEnvError(String),
    NotFound(String),
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
