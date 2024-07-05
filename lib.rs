#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod errors;

#[ink::contract]
mod az_trading_competition {
    use crate::errors::AzTradingCompetitionError;
    use ink::{
        codegen::EmitEvent,
        prelude::{string::ToString, vec, vec::Vec},
        reflect::ContractEventBase,
        storage::Mapping,
    };

    // === TYPES ===
    type Event = <AzTradingCompetition as ContractEventBase>::Type;
    type Result<T> = core::result::Result<T, AzTradingCompetitionError>;

    // === EVENTS ===
    #[ink(event)]
    pub struct TournamentCreate {
        #[ink(topic)]
        id: u64,
        start: Timestamp,
        end: Timestamp,
        entry_fee_token: AccountId,
        entry_fee_amount: Balance,
        creator: AccountId,
    }

    // === CONSTANTS ===
    // Minimum 1 hour
    const MINIMUM_DURATION: Timestamp = 3_600_000;

    // === STRUCTS ===
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Config {
        pub admin: AccountId,
        pub router: AccountId,
        pub oracle: AccountId,
        pub competition_count: u64,
    }

    #[derive(scale::Decode, scale::Encode, Debug, Clone, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Competition {
        pub id: u64,
        pub start: Timestamp,
        pub end: Timestamp,
        pub entry_fee_token: AccountId,
        pub entry_fee_amount: Balance,
        pub allowed_pools_vec: Vec<AccountId>,
        pub creator: AccountId,
    }

    // === CONTRACT ===
    #[ink(storage)]
    pub struct AzTradingCompetition {
        admin: AccountId,
        router: AccountId,
        competition_allowed_pools: Mapping<(u64, AccountId), bool>,
        competition_token_users: Mapping<(u64, AccountId, AccountId), Balance>,
        competition_count: u64,
        competitions: Mapping<u64, Competition>,
        oracle: AccountId,
    }
    impl AzTradingCompetition {
        #[ink(constructor)]
        pub fn new(router: AccountId, oracle: AccountId) -> Self {
            Self {
                admin: Self::env().caller(),
                router,
                competition_allowed_pools: Mapping::default(),
                competition_token_users: Mapping::default(),
                competition_count: 0,
                competitions: Mapping::default(),
                oracle,
            }
        }

        // === QUERIES ===
        #[ink(message)]
        pub fn config(&self) -> Config {
            Config {
                admin: self.admin,
                router: self.router,
                competition_count: self.competition_count,
                oracle: self.oracle,
            }
        }

        // === HANDLES ===
        #[ink(message)]
        pub fn create(
            &mut self,
            start: Timestamp,
            end: Timestamp,
            entry_fee_token: AccountId,
            entry_fee_amount: Balance,
        ) -> Result<Competition> {
            if self.competition_count == u64::MAX {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Max number of tournaments reached.".to_string(),
                ));
            }
            if end < start + MINIMUM_DURATION {
                return Err(AzTradingCompetitionError::UnprocessableEntity(format!(
                    "Tournament must run a minimum duration of {MINIMUM_DURATION}ms."
                )));
            }
            if entry_fee_amount == 0 {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Entry fee amount must be positive".to_string(),
                ));
            }

            let competition: Competition = Competition {
                id: self.competition_count,
                start,
                end,
                entry_fee_token,
                entry_fee_amount,
                allowed_pools_vec: vec![],
                creator: Self::env().caller(),
            };
            self.competitions
                .insert(self.competition_count, &competition);
            self.competition_count += 1;

            // emit event
            Self::emit_event(
                self.env(),
                Event::TournamentCreate(TournamentCreate {
                    id: competition.id,
                    start: competition.start,
                    end: competition.end,
                    entry_fee_token: competition.entry_fee_token,
                    entry_fee_amount: competition.entry_fee_amount,
                    creator: Self::env().caller(),
                }),
            );

            Ok(competition)
        }

        fn emit_event<EE: EmitEvent<Self>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{
            test::{default_accounts, set_caller, DefaultAccounts},
            DefaultEnvironment,
        };

        // === CONSTANTS ===
        const MOCK_ENTRY_FEE_AMOUNT: Balance = 555_555;
        const MOCK_START: Timestamp = 654_654;
        const MOCK_END: Timestamp = 754_654;

        // === HELPERS ===
        fn init() -> (DefaultAccounts<DefaultEnvironment>, AzTradingCompetition) {
            let accounts = default_accounts();
            set_caller::<DefaultEnvironment>(accounts.bob);
            let az_trading_competition =
                AzTradingCompetition::new(mock_router_address(), mock_oracle_address());
            (accounts, az_trading_competition)
        }

        fn mock_entry_fee_token() -> AccountId {
            let accounts: DefaultAccounts<DefaultEnvironment> = default_accounts();
            accounts.eve
        }

        fn mock_router_address() -> AccountId {
            let accounts: DefaultAccounts<DefaultEnvironment> = default_accounts();
            accounts.django
        }

        fn mock_oracle_address() -> AccountId {
            let accounts: DefaultAccounts<DefaultEnvironment> = default_accounts();
            accounts.frank
        }

        // === TEST QUERIES ===
        #[ink::test]
        fn test_config() {
            let (_accounts, az_trading_competition) = init();
            let config = az_trading_competition.config();
            // * it returns the config
            assert_eq!(config.admin, az_trading_competition.admin);
            assert_eq!(config.router, az_trading_competition.router);
            assert_eq!(config.oracle, mock_oracle_address());
        }

        // === TEST HANDLES ===
        #[ink::test]
        fn test_create() {
            let (_accounts, mut az_trading_competition) = init();

            // when competition_count is u64 max
            az_trading_competition.competition_count = u64::MAX;
            // * it raises an error
            let result = az_trading_competition.create(
                MOCK_START,
                MOCK_END,
                mock_entry_fee_token(),
                MOCK_ENTRY_FEE_AMOUNT,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Max number of tournaments reached.".to_string(),
                ))
            );
            // when competition_count is less than u64 max
            az_trading_competition.competition_count = u64::MAX - 1;
            // = when duration is less than or equal to MINIMUM_DURATION
            let result = az_trading_competition.create(
                MOCK_START,
                MOCK_START + MINIMUM_DURATION - 1,
                mock_entry_fee_token(),
                MOCK_ENTRY_FEE_AMOUNT,
            );
            // = * it raises an error
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(format!(
                    "Tournament must run a minimum duration of {MINIMUM_DURATION}ms."
                )))
            );
            // = when duration is greater than MINIMUM_DURATION
            // == when fee amount is zero
            let result = az_trading_competition.create(
                MOCK_START,
                MOCK_START + MINIMUM_DURATION,
                mock_entry_fee_token(),
                0,
            );
            // == * it raises an error
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Entry fee amount must be positive".to_string()
                ))
            );
            // == when fee amount is positive
            let competition_count: u64 = az_trading_competition.competition_count;
            az_trading_competition
                .create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                )
                .unwrap();
            // == * it stores the competition
            assert_eq!(
                az_trading_competition
                    .competitions
                    .get(&competition_count)
                    .is_some(),
                true
            );
            // == * it increases the competition_count by 1
            assert_eq!(
                az_trading_competition.competition_count,
                competition_count + 1
            );
        }
    }
}
