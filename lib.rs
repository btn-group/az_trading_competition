#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod errors;

#[ink::contract]
mod az_trading_competition {
    use crate::errors::AzTradingCompetitionError;
    use ink::{
        env::CallFlags,
        prelude::{string::ToString, vec, vec::Vec},
        storage::Mapping,
    };
    use openbrush::contracts::psp22::PSP22Ref;

    // === TYPES ===
    type Result<T> = core::result::Result<T, AzTradingCompetitionError>;

    // === STRUCTS ===
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Config {
        admin: AccountId,
        start: Timestamp,
        end: Timestamp,
        router: AccountId,
        allowed_pools_vec: Vec<AccountId>,
        entry_fee_token: AccountId,
        entry_fee_amount: Balance,
        oracle: AccountId,
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
        entry_fee_token: AccountId,
        entry_fee_amount: Balance,
        token_users: Mapping<(AccountId, AccountId), Balance>,
        oracle: AccountId,
    }
    impl AzTradingCompetition {
        #[ink(constructor)]
        pub fn new(
            start: Timestamp,
            end: Timestamp,
            router: AccountId,
            entry_fee_token: AccountId,
            entry_fee_amount: Balance,
            oracle: AccountId,
        ) -> Self {
            Self {
                admin: Self::env().caller(),
                start,
                end,
                router,
                allowed_pools: Mapping::default(),
                allowed_pools_vec: vec![],
                entry_fee_token,
                entry_fee_amount,
                token_users: Mapping::default(),
                oracle,
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
                entry_fee_token: self.entry_fee_token,
                entry_fee_amount: self.entry_fee_amount,
                oracle: self.oracle,
            }
        }

        // === HANDLES ===
        // Go through pools
        // check if pool is in allowed_pools
        // if not, add to allowed_pools_vec and allowed_pools
        #[ink(message)]
        pub fn add_pools(&mut self, pools: Vec<AccountId>) -> Result<()> {
            Self::authorise(self.admin, Self::env().caller())?;

            for pool in pools.iter() {
                if self.allowed_pools.get(&pool).is_none() {
                    self.allowed_pools_vec.push(*pool);
                    self.allowed_pools.insert(pool, &true);
                }
            }

            Ok(())
        }

        // Go through pools
        // check if pool is in allowed_pools
        // if not, add to allowed_pools_vec and allowed_pools
        #[ink(message)]
        pub fn remove_pools(&mut self, pools: Vec<AccountId>) -> Result<()> {
            Self::authorise(self.admin, Self::env().caller())?;

            for pool in pools.iter() {
                if self.allowed_pools.get(&pool).is_some() {
                    let index = self
                        .allowed_pools_vec
                        .iter()
                        .position(|x| x == pool)
                        .unwrap();
                    self.allowed_pools_vec.remove(index);
                    self.allowed_pools.remove(pool);
                }
            }

            Ok(())
        }

        #[ink(message)]
        pub fn register(&mut self) -> Result<()> {
            // 1. Check that time is before start
            self.competition_has_not_started()?;
            // 2. Check that user hasn't registered already
            let caller: AccountId = Self::env().caller();
            if self
                .token_users
                .get((self.entry_fee_token, caller))
                .is_some()
            {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Already registered".to_string(),
                ));
            }

            // 3. Acquire token from caller
            self.acquire_psp22(self.entry_fee_token, caller, self.entry_fee_amount)?;
            // 4. Set balance of token users
            self.token_users
                .insert((self.entry_fee_token, caller), &self.entry_fee_amount);

            Ok(())
        }

        // === PRIVATE ===
        fn acquire_psp22(&self, token: AccountId, from: AccountId, amount: Balance) -> Result<()> {
            PSP22Ref::transfer_from_builder(&token, from, self.env().account_id(), amount, vec![])
                .call_flags(CallFlags::default())
                .invoke()?;

            Ok(())
        }

        fn authorise(allowed: AccountId, received: AccountId) -> Result<()> {
            if allowed != received {
                return Err(AzTradingCompetitionError::Unauthorised);
            }

            Ok(())
        }

        fn competition_has_not_started(&self) -> Result<()> {
            let block_timestamp: Timestamp = Self::env().block_timestamp();
            if block_timestamp >= self.start {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition has started".to_string(),
                ));
            }

            Ok(())
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
            let az_trading_competition = AzTradingCompetition::new(
                MOCK_START,
                MOCK_END,
                mock_router_address(),
                mock_entry_fee_token(),
                MOCK_ENTRY_FEE_AMOUNT,
                mock_oracle_address(),
            );
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
            assert_eq!(config.start, az_trading_competition.start);
            assert_eq!(config.end, az_trading_competition.end);
            assert_eq!(config.router, az_trading_competition.router);
            assert_eq!(config.router, az_trading_competition.router);
            assert_eq!(
                config.allowed_pools_vec,
                az_trading_competition.allowed_pools_vec
            );
            assert_eq!(config.entry_fee_token, mock_entry_fee_token());
            assert_eq!(config.entry_fee_amount, MOCK_ENTRY_FEE_AMOUNT);
            assert_eq!(config.oracle, mock_oracle_address());
        }

        // === TEST HANDLES ===
        #[ink::test]
        fn test_add_pools() {
            let (accounts, mut az_trading_competition) = init();
            // when called by admin
            // = when pool is not in allowed_pools
            // = * it adds pools to allowed_pools and allowed_pools_vec
            az_trading_competition
                .add_pools(vec![accounts.django])
                .unwrap();
            assert_eq!(
                az_trading_competition
                    .allowed_pools_vec
                    .contains(&accounts.django),
                true
            );
            assert_eq!(
                az_trading_competition
                    .allowed_pools
                    .get(&accounts.django)
                    .is_some(),
                true
            );
            // = when multiple pools are provided
            // = * it adds pools that haven't been added already
            az_trading_competition
                .add_pools(vec![accounts.django, accounts.alice])
                .unwrap();
            assert_eq!(
                az_trading_competition
                    .allowed_pools_vec
                    .contains(&accounts.alice),
                true
            );
            assert_eq!(
                az_trading_competition
                    .allowed_pools
                    .get(&accounts.alice)
                    .is_some(),
                true
            );
            // = * it ignores the pools have already been added
            assert_eq!(
                az_trading_competition
                    .allowed_pools_vec
                    .contains(&accounts.django),
                true
            );
            assert_eq!(az_trading_competition.allowed_pools_vec.len(), 2);

            // when called by non-admin
            set_caller::<DefaultEnvironment>(accounts.django);
            // * it raises an error
            let result = az_trading_competition.add_pools(vec![accounts.django, accounts.alice]);
            assert_eq!(result, Err(AzTradingCompetitionError::Unauthorised));
        }

        #[ink::test]
        fn test_remove_pools() {
            let (accounts, mut az_trading_competition) = init();
            // when called by admin
            // = when pool is in allowed_pools
            az_trading_competition
                .add_pools(vec![accounts.django])
                .unwrap();
            // == when pool being removed is in allowed_pools
            // == * it removes the pool from allowed_pools_vec
            az_trading_competition
                .remove_pools(vec![accounts.django, accounts.alice])
                .unwrap();
            assert_eq!(
                az_trading_competition
                    .allowed_pools_vec
                    .contains(&accounts.django),
                false
            );
            // == * it removes the pool from allowed_pools
            assert_eq!(
                az_trading_competition
                    .allowed_pools
                    .get(&accounts.django)
                    .is_none(),
                true
            );
            assert_eq!(az_trading_competition.allowed_pools_vec.len(), 0);
            // when called by non-admin
            set_caller::<DefaultEnvironment>(accounts.django);
            // * it raises an error
            let result = az_trading_competition.remove_pools(vec![accounts.django, accounts.alice]);
            assert_eq!(result, Err(AzTradingCompetitionError::Unauthorised));
        }

        #[ink::test]
        fn test_register() {
            let (accounts, mut az_trading_competition) = init();

            // when competition has started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                az_trading_competition.start,
            );
            // * it raises an error
            let result = az_trading_competition.register();
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition has started".to_string(),
                ))
            );
            // when competition has not started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                az_trading_competition.start - 1,
            );
            // = when user has registered already
            az_trading_competition.token_users.insert(
                (az_trading_competition.entry_fee_token, accounts.bob),
                &az_trading_competition.entry_fee_amount,
            );
            // = * it raises an error
            let result = az_trading_competition.register();
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Already registered".to_string(),
                ))
            );
        }
    }
}
