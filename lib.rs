#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod errors;

#[ink::contract]
mod az_trading_competition {
    use crate::errors::AzTradingCompetitionError;
    use bs58;
    use ink::{
        codegen::EmitEvent,
        env::CallFlags,
        prelude::{string::ToString, vec, vec::Vec},
        reflect::ContractEventBase,
        storage::Mapping,
    };
    use openbrush::contracts::psp22::PSP22Ref;

    // === TYPES ===
    type Event = <AzTradingCompetition as ContractEventBase>::Type;
    type Result<T> = core::result::Result<T, AzTradingCompetitionError>;

    // === EVENTS ===
    #[ink(event)]
    pub struct CompetitionsCreate {
        #[ink(topic)]
        id: u64,
        start: Timestamp,
        end: Timestamp,
        entry_fee_token: AccountId,
        entry_fee_amount: Balance,
        creator: AccountId,
    }

    #[ink(event)]
    pub struct PoolsAdd {
        #[ink(topic)]
        id: u64,
        pools: Vec<AccountId>,
    }

    #[ink(event)]
    pub struct PoolsRemove {
        #[ink(topic)]
        id: u64,
        pools: Vec<AccountId>,
    }

    #[ink(event)]
    pub struct Register {
        #[ink(topic)]
        id: u64,
        user: AccountId,
    }

    // === CONSTANTS ===
    // Minimum 1 hour
    const MINIMUM_DURATION: Timestamp = 3_600_000;

    // === STATICS ===
    static TOKEN_TO_DIA_PRICE_SYMBOL_COMBOS: &[(&str, &str)] = &[
        (
            "5CtuFVgEUz13SFPVY6s2cZrnLDEkxQXc19aXrNARwEBeCXgg",
            "AZERO/USD",
        ),
        (
            "5EoFQd36196Duo6fPTz2MWHXRzwTJcyETHyCyaB3rb61Xo2u",
            "ETH/USD",
        ),
        (
            "5FYFojNCJVFR2bBNKfAePZCa72ZcVX5yeTv8K9bzeUo8D83Z",
            "USDC/USD",
        ),
        (
            "5Et3dDcXUiThrBCot7g65k3oDSicGy4qC82cq9f911izKNtE",
            "USDT/USD",
        ),
    ];

    // === STRUCTS ===
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Config {
        pub admin: AccountId,
        pub router: AccountId,
        pub oracle: AccountId,
        pub competitions_count: u64,
        pub dia_price_symbols_vec: Vec<(AccountId, String)>,
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
        pub user_count: u64,
    }

    // === CONTRACT ===
    #[ink(storage)]
    pub struct AzTradingCompetition {
        admin: AccountId,
        router: AccountId,
        competition_allowed_pools: Mapping<(u64, AccountId), bool>,
        competition_token_users: Mapping<(u64, AccountId, AccountId), Balance>,
        competitions_count: u64,
        competitions: Mapping<u64, Competition>,
        oracle: AccountId,
        dia_price_symbols: Mapping<AccountId, String>,
    }
    impl AzTradingCompetition {
        #[ink(constructor)]
        pub fn new(router: AccountId, oracle: AccountId) -> Self {
            let mut x = Self {
                admin: Self::env().caller(),
                router,
                competition_allowed_pools: Mapping::default(),
                competition_token_users: Mapping::default(),
                competitions_count: 0,
                competitions: Mapping::default(),
                oracle,
                dia_price_symbols: Mapping::default(),
            };
            for token_dia_price_symbol_combo in TOKEN_TO_DIA_PRICE_SYMBOL_COMBOS.iter() {
                x.dia_price_symbols.insert(
                    Self::convert_string_to_account_id(token_dia_price_symbol_combo.0),
                    &token_dia_price_symbol_combo.1.to_string(),
                );
            }
            x
        }

        // === QUERIES ===
        #[ink(message)]
        pub fn config(&self) -> Config {
            Config {
                admin: self.admin,
                router: self.router,
                competitions_count: self.competitions_count,
                oracle: self.oracle,
                dia_price_symbols_vec: TOKEN_TO_DIA_PRICE_SYMBOL_COMBOS
                    .iter()
                    .map(|token_dia_price_symbol_combo| {
                        (
                            Self::convert_string_to_account_id(token_dia_price_symbol_combo.0),
                            token_dia_price_symbol_combo.1.to_string(),
                        )
                    })
                    .collect(),
            }
        }

        #[ink(message)]
        pub fn competitions_show(&self, id: u64) -> Result<Competition> {
            self.competitions
                .get(id)
                .ok_or(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
        }

        // === HANDLES ===
        #[ink(message)]
        pub fn competitions_create(
            &mut self,
            start: Timestamp,
            end: Timestamp,
            entry_fee_token: AccountId,
            entry_fee_amount: Balance,
        ) -> Result<Competition> {
            if self.competitions_count == u64::MAX {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Max number of competitions reached.".to_string(),
                ));
            }
            if end < start + MINIMUM_DURATION {
                return Err(AzTradingCompetitionError::UnprocessableEntity(format!(
                    "Competition must run a minimum duration of {MINIMUM_DURATION}ms."
                )));
            }
            if entry_fee_amount == 0 {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Entry fee amount must be positive".to_string(),
                ));
            }
            if self.dia_price_symbols.get(entry_fee_token).is_none() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Entry fee token is not permitted.".to_string(),
                ));
            }

            let competition: Competition = Competition {
                id: self.competitions_count,
                start,
                end,
                entry_fee_token,
                entry_fee_amount,
                allowed_pools_vec: vec![],
                creator: Self::env().caller(),
                user_count: 0,
            };
            self.competitions
                .insert(self.competitions_count, &competition);
            self.competitions_count += 1;

            // emit event
            Self::emit_event(
                self.env(),
                Event::CompetitionsCreate(CompetitionsCreate {
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

        // Go through pools
        // check if pool is in allowed_pools
        // if not, add to allowed_pools_vec and allowed_pools
        #[ink(message)]
        pub fn pools_add(&mut self, id: u64, pools: Vec<AccountId>) -> Result<Competition> {
            let mut competition: Competition = self.competitions_show(id)?;
            if competition.user_count > 0 {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Pools can't be added when registrations are present.".to_string(),
                ));
            }
            Self::authorise(competition.creator, Self::env().caller())?;

            for pool in pools.iter() {
                if self.competition_allowed_pools.get((id, pool)).is_none() {
                    competition.allowed_pools_vec.push(*pool);
                    self.competition_allowed_pools.insert((id, pool), &true);
                }
            }
            self.competitions.insert(competition.id, &competition);

            // emit event
            Self::emit_event(
                self.env(),
                Event::PoolsAdd(PoolsAdd {
                    id: competition.id,
                    pools,
                }),
            );

            Ok(competition)
        }

        // Go through pools
        // check if pool is in allowed_pools
        // if so, remove from allowed_pools_vec and allowed_pools
        #[ink(message)]
        pub fn pools_remove(&mut self, id: u64, pools: Vec<AccountId>) -> Result<()> {
            let mut competition: Competition = self.competitions_show(id)?;
            if competition.user_count > 0 {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Pools can't be removed when registrations are present.".to_string(),
                ));
            }
            Self::authorise(competition.creator, Self::env().caller())?;

            for pool in pools.iter() {
                if self.competition_allowed_pools.get((id, pool)).is_some() {
                    let index = competition
                        .allowed_pools_vec
                        .iter()
                        .position(|x| x == pool)
                        .unwrap();
                    competition.allowed_pools_vec.remove(index);
                    self.competition_allowed_pools.remove((id, pool));
                }
            }
            self.competitions.insert(competition.id, &competition);

            // emit event
            Self::emit_event(
                self.env(),
                Event::PoolsRemove(PoolsRemove {
                    id: competition.id,
                    pools,
                }),
            );

            Ok(())
        }

        #[ink(message)]
        pub fn register(&mut self, id: u64) -> Result<()> {
            let mut competition: Competition = self.competitions_show(id)?;
            // 1. Check that time is before start
            self.competition_has_not_started(competition.start)?;
            // 2. Check that user hasn't registered already
            let caller: AccountId = Self::env().caller();
            if self
                .competition_token_users
                .get((id, competition.entry_fee_token, caller))
                .is_some()
            {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Already registered".to_string(),
                ));
            }

            // 3. Acquire token from caller
            self.acquire_psp22(
                competition.entry_fee_token,
                caller,
                competition.entry_fee_amount,
            )?;
            // 4. Set balance of token users
            self.competition_token_users.insert(
                (id, competition.entry_fee_token, caller),
                &competition.entry_fee_amount,
            );
            // 5. Increase competition.user_count
            competition.user_count += 1;
            self.competitions.insert(competition.id, &competition);

            // emit event
            Self::emit_event(self.env(), Event::Register(Register { id, user: caller }));

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

        fn competition_has_not_started(&self, start: Timestamp) -> Result<()> {
            if Self::env().block_timestamp() >= start {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition has started".to_string(),
                ));
            }

            Ok(())
        }

        fn convert_string_to_account_id(account_str: &str) -> AccountId {
            let mut output = vec![0xFF; 35];
            bs58::decode(account_str).onto(&mut output).unwrap();
            let cut_address_vec: Vec<_> = output.drain(1..33).collect();
            let mut array = [0; 32];
            let bytes = &cut_address_vec[..array.len()];
            array.copy_from_slice(bytes);
            let account_id: AccountId = array.into();
            account_id
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
            AzTradingCompetition::convert_string_to_account_id(
                "5CtuFVgEUz13SFPVY6s2cZrnLDEkxQXc19aXrNARwEBeCXgg",
            )
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
            assert_eq!(
                config.dia_price_symbols_vec,
                TOKEN_TO_DIA_PRICE_SYMBOL_COMBOS
                    .iter()
                    .map(|token_dia_price_symbol_combo| (
                        AzTradingCompetition::convert_string_to_account_id(
                            token_dia_price_symbol_combo.0
                        ),
                        token_dia_price_symbol_combo.1.to_string()
                    ))
                    .collect::<Vec<_>>()
            );
        }

        // === TEST HANDLES ===
        #[ink::test]
        fn test_competitions_create() {
            let (_accounts, mut az_trading_competition) = init();

            // when competitions_count is u64 max
            az_trading_competition.competitions_count = u64::MAX;
            // * it raises an error
            let result = az_trading_competition.competitions_create(
                MOCK_START,
                MOCK_END,
                mock_entry_fee_token(),
                MOCK_ENTRY_FEE_AMOUNT,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Max number of competitions reached.".to_string(),
                ))
            );
            // when competitions_count is less than u64 max
            az_trading_competition.competitions_count = u64::MAX - 1;
            // = when duration is less than or equal to MINIMUM_DURATION
            let result = az_trading_competition.competitions_create(
                MOCK_START,
                MOCK_START + MINIMUM_DURATION - 1,
                mock_entry_fee_token(),
                MOCK_ENTRY_FEE_AMOUNT,
            );
            // = * it raises an error
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(format!(
                    "Competition must run a minimum duration of {MINIMUM_DURATION}ms."
                )))
            );
            // = when duration is greater than MINIMUM_DURATION
            // == when fee amount is zero
            let result = az_trading_competition.competitions_create(
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
            let competitions_count: u64 = az_trading_competition.competitions_count;
            // === when fee token doesn't have a dia price symbol
            let result = az_trading_competition.competitions_create(
                MOCK_START,
                MOCK_START + MINIMUM_DURATION,
                mock_oracle_address(),
                MOCK_ENTRY_FEE_AMOUNT,
            );
            // === * it raises an error
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Entry fee token is not permitted.".to_string()
                ))
            );
            // === when fee token has a dia price symbol
            az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                )
                .unwrap();
            // === * it stores the competition
            assert_eq!(
                az_trading_competition
                    .competitions
                    .get(&competitions_count)
                    .is_some(),
                true
            );
            // === * it increases the competitions_count by 1
            assert_eq!(
                az_trading_competition.competitions_count,
                competitions_count + 1
            );
        }

        #[ink::test]
        fn test_pools_add() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.pools_add(0, vec![accounts.django]);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
            );
            // when competition exist
            az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                )
                .unwrap();
            // = when competition has registrants
            let mut competition: Competition = az_trading_competition.competitions_show(0).unwrap();
            competition.user_count = 1;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // = * it raises an error
            let result = az_trading_competition.pools_add(0, vec![accounts.django]);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Pools can't be added when registrations are present.".to_string(),
                ))
            );
            // = when competition has no registrants
            competition.user_count = 0;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // == when called by a non-creator of the competition
            set_caller::<DefaultEnvironment>(accounts.django);
            // == * it raises an error
            let result = az_trading_competition.pools_add(0, vec![accounts.django]);
            assert_eq!(result, Err(AzTradingCompetitionError::Unauthorised));
            // == when called by the creator of the competition
            set_caller::<DefaultEnvironment>(competition.creator);
            // === when pool is not in allowed_pools
            // === * it adds pools to allowed_pools and allowed_pools_vec
            az_trading_competition
                .pools_add(0, vec![accounts.django])
                .unwrap();
            competition = az_trading_competition.competitions_show(0).unwrap();
            assert_eq!(
                competition.allowed_pools_vec.contains(&accounts.django),
                true
            );
            assert_eq!(
                az_trading_competition
                    .competition_allowed_pools
                    .get(&(0, accounts.django))
                    .is_some(),
                true
            );
            // === when multiple pools are provided
            // === * it adds pools that haven't been added already
            az_trading_competition
                .pools_add(0, vec![accounts.django, accounts.alice])
                .unwrap();
            competition = az_trading_competition.competitions_show(0).unwrap();
            assert_eq!(
                competition.allowed_pools_vec.contains(&accounts.alice),
                true
            );
            assert_eq!(
                az_trading_competition
                    .competition_allowed_pools
                    .get(&(0, accounts.alice))
                    .is_some(),
                true
            );
            // === * it ignores the pools have already been added
            assert_eq!(
                competition.allowed_pools_vec.contains(&accounts.django),
                true
            );
            assert_eq!(competition.allowed_pools_vec.len(), 2);
        }

        #[ink::test]
        fn test_pools_remove() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.pools_remove(0, vec![accounts.django]);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
            );
            // when competition exist
            az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                )
                .unwrap();
            // = when competition has registrants
            let mut competition: Competition = az_trading_competition.competitions_show(0).unwrap();
            competition.user_count = 1;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // = * it raises an error
            let result = az_trading_competition.pools_remove(0, vec![accounts.django]);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Pools can't be removed when registrations are present.".to_string(),
                ))
            );
            // = when competition has no registrants
            competition.user_count = 0;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // == when called by a non-creator of the competition
            set_caller::<DefaultEnvironment>(accounts.django);
            // == * it raises an error
            let result = az_trading_competition.pools_remove(0, vec![accounts.django]);
            assert_eq!(result, Err(AzTradingCompetitionError::Unauthorised));
            // == when called by the creator of the competition
            set_caller::<DefaultEnvironment>(competition.creator);
            // === when pool is in allowed pools
            az_trading_competition
                .pools_add(0, vec![accounts.django, accounts.alice])
                .unwrap();
            az_trading_competition
                .pools_remove(0, vec![accounts.django])
                .unwrap();
            // === * it removes the pool from competition_allowed_pools
            assert_eq!(
                az_trading_competition
                    .competition_allowed_pools
                    .get((0, accounts.django))
                    .is_none(),
                true
            );
            // === * it removes the pool from competition.allowed_pools_vec
            competition = az_trading_competition.competitions_show(0).unwrap();
            assert_eq!(competition.allowed_pools_vec, vec![accounts.alice])
        }

        #[ink::test]
        fn test_register() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.pools_remove(0, vec![accounts.django]);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
            );
            // when competition exist
            az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                )
                .unwrap();

            // when competition has started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START);
            // * it raises an error
            let result = az_trading_competition.register(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition has started".to_string(),
                ))
            );
            // when competition has not started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START - 1);
            // = when user has registered already
            az_trading_competition.competition_token_users.insert(
                (0, mock_entry_fee_token(), accounts.bob),
                &MOCK_ENTRY_FEE_AMOUNT,
            );
            // = * it raises an error
            let result = az_trading_competition.register(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Already registered".to_string(),
                ))
            );
            // == the rest needs to be done in integration tests
        }
    }
}
