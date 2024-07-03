#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod az_trading_competition {
    use ink::prelude::{vec, vec::Vec};
    use ink::storage::Mapping;

    // === STRUCTS ===
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Config {
        admin: AccountId,
        start: Timestamp,
        end: Timestamp,
        router: AccountId,
        allowed_pools_vec: Vec<AccountId>,
    }

    // === CONTRACT ===
    #[ink(storage)]
    pub struct AzTradingCompetition {
        admin: AccountId,
        start: Timestamp,
        end: Timestamp,
        router: AccountId,
        allowed_pools: Mapping<AccountId, bool>,
        allowed_pools_vec: Vec<AccountId>,
    }

    impl AzTradingCompetition {
        #[ink(constructor)]
        pub fn new(start: Timestamp, end: Timestamp, router: AccountId) -> Self {
            Self {
                admin: Self::env().caller(),
                start,
                end,
                router,
                allowed_pools: Mapping::default(),
                allowed_pools_vec: vec![],
            }
        }

        // === QUERIES ===
        #[ink(message)]
        pub fn config(&self) -> Config {
            Config {
                admin: self.admin,
                start: self.start,
                end: self.end,
                router: self.router,
                allowed_pools_vec: self.allowed_pools_vec.clone(),
            }
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
        const MOCK_START: Timestamp = 654_654;
        const MOCK_END: Timestamp = 754_654;

        // === HELPERS ===
        fn init() -> (DefaultAccounts<DefaultEnvironment>, AzTradingCompetition) {
            let accounts = default_accounts();
            set_caller::<DefaultEnvironment>(accounts.bob);
            let az_trading_competition =
                AzTradingCompetition::new(MOCK_START, MOCK_END, mock_router_address());
            (accounts, az_trading_competition)
        }

        fn mock_router_address() -> AccountId {
            let accounts: DefaultAccounts<DefaultEnvironment> = default_accounts();
            accounts.django
        }

        // === TEST QUERIES ===
        #[ink::test]
        fn test_config() {
            let (_accounts, az_trading_competition) = init();
            let config = az_trading_competition.config();
            // * it returns the config
            assert_eq!(config.admin, az_trading_competition.admin);
            assert_eq!(config.start, az_trading_competition.start);
            assert_eq!(config.end, az_trading_competition.end);
            assert_eq!(config.router, az_trading_competition.router);
            assert_eq!(config.router, az_trading_competition.router);
            assert_eq!(
                config.allowed_pools_vec,
                az_trading_competition.allowed_pools_vec
            );
        }
    }
}
