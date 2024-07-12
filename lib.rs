#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod errors;

#[ink::contract]
mod az_trading_competition {
    use crate::errors::{AzTradingCompetitionError, RouterError};
    use ink::{
        codegen::EmitEvent,
        env::call::{build_call, ExecutionInput, Selector},
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
    pub struct PayoutStructureUpdate {
        #[ink(topic)]
        id: u64,
        payout_structure_numerators: Vec<(u16, u16)>,
    }

    #[ink(event)]
    pub struct Register {
        #[ink(topic)]
        id: u64,
        user: AccountId,
    }

    #[ink(event)]
    pub struct Swap {
        id: u64,
        user: AccountId,
        in_token: AccountId,
        in_amount: Balance,
        out_token: AccountId,
        out_amount: Balance,
    }

    // === CONSTANTS ===
    // Minimum 1 hour
    const MINIMUM_DURATION: Timestamp = 3_600_000;
    const PAYOUT_STRUCTURE_DENOMINATOR: u16 = 10_000;
    const VALID_DIA_PRICE_SYMBOLS: &[&str] = &["AZERO/USD", "ETH/USD", "USDC/USD", "USDT/USD"];

    // === STRUCTS ===
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Config {
        pub admin: AccountId,
        pub router: AccountId,
        pub oracle: AccountId,
        pub competitions_count: u64,
        pub dia_price_symbols_vec: Vec<(AccountId, String)>,
        pub allowed_pair_token_combinations_vec: Vec<(AccountId, AccountId)>,
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
        pub creator: AccountId,
        pub payout_places: u16,
        pub payout_structure_numerator_sum: u16,
        pub user_count: u32,
    }

    // === CONTRACT ===
    #[ink(storage)]
    pub struct AzTradingCompetition {
        admin: AccountId,
        router: AccountId,
        competition_payout_structure_numerators: Mapping<(u64, u16), u16>,
        competition_token_users: Mapping<(u64, AccountId, AccountId), Balance>,
        competitions_count: u64,
        competitions: Mapping<u64, Competition>,
        oracle: AccountId,
        dia_price_symbols: Mapping<AccountId, String>,
        dia_price_symbols_vec: Vec<(AccountId, String)>,
        allowed_pair_token_combinations: Mapping<AccountId, Vec<AccountId>>,
        allowed_pair_token_combinations_vec: Vec<(AccountId, AccountId)>,
    }
    impl AzTradingCompetition {
        #[ink(constructor)]
        pub fn new(
            router: AccountId,
            oracle: AccountId,
            dia_price_symbols_vec: Vec<(AccountId, String)>,
            allowed_pair_token_combinations_vec: Vec<(AccountId, AccountId)>,
        ) -> Result<Self> {
            let mut x = Self {
                admin: Self::env().caller(),
                router,
                competition_payout_structure_numerators: Mapping::default(),
                competition_token_users: Mapping::default(),
                competitions_count: 0,
                competitions: Mapping::default(),
                oracle,
                dia_price_symbols: Mapping::default(),
                dia_price_symbols_vec: dia_price_symbols_vec.clone(),
                allowed_pair_token_combinations: Mapping::default(),
                allowed_pair_token_combinations_vec: allowed_pair_token_combinations_vec.clone(),
            };
            for token_dia_price_symbol_combo in dia_price_symbols_vec.iter() {
                if VALID_DIA_PRICE_SYMBOLS.contains(&&token_dia_price_symbol_combo.1[..]) {
                    x.dia_price_symbols.insert(
                        token_dia_price_symbol_combo.0,
                        &token_dia_price_symbol_combo.1,
                    );
                } else {
                    return Err(AzTradingCompetitionError::UnprocessableEntity(
                        "Invalid DIA price symbol.".to_string(),
                    ));
                }
            }
            for allowed_pair_token_combination in allowed_pair_token_combinations_vec.iter() {
                if x.dia_price_symbols
                    .get(allowed_pair_token_combination.0)
                    .is_none()
                    || x.dia_price_symbols
                        .get(allowed_pair_token_combination.1)
                        .is_none()
                {
                    return Err(AzTradingCompetitionError::UnprocessableEntity(
                        "Invalid pair token combinations.".to_string(),
                    ));
                } else {
                    if let Some(mut allowed_to_tokens) = x
                        .allowed_pair_token_combinations
                        .get(allowed_pair_token_combination.0)
                    {
                        allowed_to_tokens.push(allowed_pair_token_combination.1);
                        x.allowed_pair_token_combinations
                            .insert(allowed_pair_token_combination.0, &allowed_to_tokens);
                    } else {
                        x.allowed_pair_token_combinations.insert(
                            allowed_pair_token_combination.0,
                            &vec![allowed_pair_token_combination.1],
                        );
                    }
                    if let Some(mut allowed_to_tokens) = x
                        .allowed_pair_token_combinations
                        .get(allowed_pair_token_combination.1)
                    {
                        allowed_to_tokens.push(allowed_pair_token_combination.0);
                        x.allowed_pair_token_combinations
                            .insert(allowed_pair_token_combination.1, &allowed_to_tokens);
                    } else {
                        x.allowed_pair_token_combinations.insert(
                            allowed_pair_token_combination.1,
                            &vec![allowed_pair_token_combination.0],
                        );
                    }
                }
            }
            Ok(x)
        }

        // === QUERIES ===
        #[ink(message)]
        pub fn config(&self) -> Config {
            Config {
                admin: self.admin,
                router: self.router,
                competitions_count: self.competitions_count,
                oracle: self.oracle,
                dia_price_symbols_vec: self.dia_price_symbols_vec.clone(),
                allowed_pair_token_combinations_vec: self
                    .allowed_pair_token_combinations_vec
                    .clone(),
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
                payout_places: 0,
                payout_structure_numerator_sum: 0,
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

        #[ink(message)]
        pub fn competition_payout_structure_numerators_update(
            &mut self,
            id: u64,
            payout_structure_numerators: Vec<(u16, u16)>,
        ) -> Result<u16> {
            let caller: AccountId = Self::env().caller();
            let mut competition: Competition = self.competitions_show(id)?;
            Self::authorise(competition.creator, caller)?;
            self.competition_has_not_started(competition.start)?;
            if competition.user_count > 0 {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Unable to change when registrants present.".to_string(),
                ));
            }

            // Do the validations first as the inserts sustain in tests
            // even if there is an error
            let mut positions: Vec<u16> = vec![];
            for payout_structure_numerator in payout_structure_numerators.iter() {
                let position: u16 = payout_structure_numerator.0;
                positions.push(position);
                let numerator: u16 = payout_structure_numerator.1;
                let previous_numerator: u16 = self
                    .competition_payout_structure_numerators
                    .get((id, position))
                    .unwrap_or(0);

                // 1. Check that the position before is present
                if position > 0
                    && self
                        .competition_payout_structure_numerators
                        .get((id, position - 1))
                        .is_none()
                    && !positions.contains(&(position - 1))
                {
                    return Err(AzTradingCompetitionError::UnprocessableEntity(
                        "Position must come after a present position.".to_string(),
                    ));
                }
                // 2. Check that numerator is positive
                if numerator == 0 {
                    return Err(AzTradingCompetitionError::UnprocessableEntity(
                        "Numerator must be positive.".to_string(),
                    ));
                }

                // 3. Update payout_places if possible
                if position >= competition.payout_places {
                    competition.payout_places = position + 1
                }
                // 4. Add to numerator sum
                competition.payout_structure_numerator_sum += numerator;
                // 5. Subtract previous numerator if present
                competition.payout_structure_numerator_sum -= previous_numerator;
            }
            // 6. Check that numerator sum is less than or equal to denominator
            if competition.payout_structure_numerator_sum > PAYOUT_STRUCTURE_DENOMINATOR {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Numerator is greater than denominator.".to_string(),
                ));
            }

            // 7. Save
            for payout_structure_numerator in payout_structure_numerators.iter() {
                let position: u16 = payout_structure_numerator.0;
                let numerator: u16 = payout_structure_numerator.1;
                self.competition_payout_structure_numerators
                    .insert((id, position), &numerator);
            }

            // 8. Save competition
            self.competitions.insert(id, &competition);

            // Emit event
            Self::emit_event(
                self.env(),
                Event::PayoutStructureUpdate(PayoutStructureUpdate {
                    id,
                    payout_structure_numerators,
                }),
            );

            Ok(competition.payout_structure_numerator_sum)
        }

        #[ink(message)]
        pub fn increase_allowance_for_router(
            &mut self,
            token: AccountId,
            amount: Balance,
        ) -> Result<()> {
            PSP22Ref::increase_allowance_builder(&token, self.router, amount)
                .call_flags(CallFlags::default())
                .invoke()?;

            Ok(())
        }

        #[ink(message)]
        pub fn register(&mut self, id: u64) -> Result<()> {
            let mut competition: Competition = self.competitions_show(id)?;
            // 1. Validate that time is before start
            self.competition_has_not_started(competition.start)?;
            // 2. Validate that user hasn't registered already
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

        #[ink(message)]
        pub fn swap_exact_tokens_for_tokens(
            &mut self,
            id: u64,
            amount_in: u128,
            amount_out_min: u128,
            path: Vec<AccountId>,
            deadline: u64,
        ) -> Result<()> {
            let competition: Competition = self.competitions_show(id)?;
            if path.is_empty() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Path is empty.".to_string(),
                ));
            }

            let in_token = path[0];
            let out_token = path[path.len() - 1];
            // 1. Validate that competition is in progress
            self.competition_is_in_progress(competition.clone())?;
            // 2. Validate that user is part of the competition
            let caller: AccountId = Self::env().caller();
            if self
                .competition_token_users
                .get((id, competition.entry_fee_token, caller))
                .is_none()
            {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "You are not registered for this competition.".to_string(),
                ));
            }
            // 3. Validate that path is valid
            let mut previous_token: Option<AccountId> = None;
            for token in path.iter() {
                if previous_token.is_some() {
                    let mut valid = false;
                    if let Some(to_tokens) = self
                        .allowed_pair_token_combinations
                        .get(previous_token.unwrap())
                    {
                        if to_tokens.iter().any(|&i| i == *token) {
                            valid = true
                        }
                    }
                    if !valid {
                        return Err(AzTradingCompetitionError::UnprocessableEntity(
                            "Path is invalid.".to_string(),
                        ));
                    }
                }
                previous_token = Some(*token)
            }
            // 4. Check that user has enough to cover amount_in
            if amount_in
                > self
                    .competition_token_users
                    .get((id, in_token, caller))
                    .unwrap_or(0)
            {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Insufficient balance.".to_string(),
                ));
            }
            // 5. Check that deadline is less than or equal to end
            if deadline > competition.end {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Deadline is after competition end.".to_string(),
                ));
            }

            // 6. Call router
            const SWAP_EXACT_TOKENS_FOR_TOKENS_SELECTOR: [u8; 4] =
                ink::selector_bytes!("swap_exact_tokens_for_tokens");
            let result_of_swaps: Vec<u128> = build_call::<Environment>()
                .call(self.router)
                .exec_input(
                    ExecutionInput::new(Selector::new(SWAP_EXACT_TOKENS_FOR_TOKENS_SELECTOR))
                        .push_arg(amount_in)
                        .push_arg(amount_out_min)
                        .push_arg(path.clone())
                        .push_arg(self.env().account_id())
                        .push_arg(deadline),
                )
                .returns::<core::result::Result<Vec<u128>, RouterError>>()
                .invoke()?;
            let out_amount: u128 = result_of_swaps[result_of_swaps.len() - 1];
            // 7. Adjust user balances
            // Decrease amount_in for competition token user
            let in_competition_token_user_balance: Balance = self
                .competition_token_users
                .get((id, in_token, caller))
                .unwrap_or(0);
            self.competition_token_users.insert(
                (id, in_token, caller),
                &(in_competition_token_user_balance - amount_in),
            );
            // Increase received amount for competition token user
            let out_competition_token_user_balance: Balance = self
                .competition_token_users
                .get((id, out_token, caller))
                .unwrap_or(0);
            self.competition_token_users.insert(
                (id, out_token, caller),
                &(out_competition_token_user_balance + out_amount),
            );

            // emit event
            Self::emit_event(
                self.env(),
                Event::Swap(Swap {
                    id,
                    user: caller,
                    in_token,
                    in_amount: amount_in,
                    out_token,
                    out_amount,
                }),
            );

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

        fn competition_is_in_progress(&self, competition: Competition) -> Result<()> {
            if Self::env().block_timestamp() < competition.start
                || Self::env().block_timestamp() > competition.end
            {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition isn't in progress.".to_string(),
                ));
            }

            Ok(())
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
            let az_trading_competition = AzTradingCompetition::new(
                mock_router_address(),
                mock_oracle_address(),
                mock_token_to_dia_price_symbol_combos(),
                mock_allowed_pair_token_combinations(),
            );
            (accounts, az_trading_competition.expect("REASON"))
        }

        fn mock_token_to_dia_price_symbol_combos() -> Vec<(AccountId, String)> {
            vec![
                (
                    AccountId::try_from(*b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
                    "AZERO/USD".to_string(),
                ),
                (
                    AccountId::try_from(*b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap(),
                    "ETH/USD".to_string(),
                ),
                (
                    AccountId::try_from(*b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").unwrap(),
                    "USDC/USD".to_string(),
                ),
                (
                    AccountId::try_from(*b"tttttttttttttttttttttttttttttttt").unwrap(),
                    "USDT/USD".to_string(),
                ),
            ]
        }

        fn mock_allowed_pair_token_combinations() -> Vec<(AccountId, AccountId)> {
            vec![
                // WAZERO/USDC
                (
                    AccountId::try_from(*b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
                    AccountId::try_from(*b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").unwrap(),
                ),
                // WAZERO/ETH
                (
                    AccountId::try_from(*b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
                    AccountId::try_from(*b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap(),
                ),
                // USDC/USDT
                (
                    AccountId::try_from(*b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").unwrap(),
                    AccountId::try_from(*b"tttttttttttttttttttttttttttttttt").unwrap(),
                ),
            ]
        }

        fn mock_entry_fee_token() -> AccountId {
            AccountId::try_from(*b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").unwrap()
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
                mock_token_to_dia_price_symbol_combos()
            );
            assert_eq!(
                config.allowed_pair_token_combinations_vec,
                mock_allowed_pair_token_combinations()
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
        fn test_competition_payout_structure_numerators_update() {
            let (accounts, mut az_trading_competition) = init();
            let mut payout_structure_numerators: Vec<(u16, u16)> = vec![(0, 1)];
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.competition_payout_structure_numerators_update(
                0,
                payout_structure_numerators.clone(),
            );
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
            // = when called by non-creator
            set_caller::<DefaultEnvironment>(accounts.charlie);
            // = * it raises an error
            let result = az_trading_competition.competition_payout_structure_numerators_update(
                0,
                payout_structure_numerators.clone(),
            );
            assert_eq!(result, Err(AzTradingCompetitionError::Unauthorised));
            // = when called by creator
            set_caller::<DefaultEnvironment>(accounts.bob);
            // == when competition has started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START);
            let result = az_trading_competition.competition_payout_structure_numerators_update(
                0,
                payout_structure_numerators.clone(),
            );
            // == * it raises an error
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition has started".to_string(),
                ))
            );
            // == when competition has not started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START - 1);
            // === when competition has registrants
            let mut competition: Competition = az_trading_competition.competitions.get(0).unwrap();
            competition.user_count = 1;
            az_trading_competition.competitions.insert(0, &competition);
            let result = az_trading_competition.competition_payout_structure_numerators_update(
                0,
                payout_structure_numerators.clone(),
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Unable to change when registrants present.".to_string(),
                ))
            );
            // === when competition does not have registrants
            competition.user_count = 0;
            az_trading_competition.competitions.insert(0, &competition);
            // ==== when a payout_structure_numerator is greater than zero and the position before does not have a numerator set
            // ==== * it raises an error
            payout_structure_numerators = vec![(0, 1), (2, 1)];
            let result = az_trading_competition
                .competition_payout_structure_numerators_update(0, payout_structure_numerators);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Position must come after a present position.".to_string(),
                ))
            );
            // ==== when all payout_structure_numerators have a zero position or have a position before with a numerator set
            payout_structure_numerators =
                vec![(0, 1), (1, 2), (2, PAYOUT_STRUCTURE_DENOMINATOR - 2 - 1)];
            // ===== when a numerator is zero
            payout_structure_numerators = vec![
                (0, 1),
                (1, 2),
                (2, PAYOUT_STRUCTURE_DENOMINATOR - 2 - 1),
                (3, 0),
            ];
            // ===== * it raises an error
            let result = az_trading_competition.competition_payout_structure_numerators_update(
                0,
                payout_structure_numerators.clone(),
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Numerator must be positive.".to_string(),
                ))
            );
            // ====== when all numerators are positive
            // ======= when new numerators causes the sum of numerators to be larger than denominator
            payout_structure_numerators = vec![
                (0, 1),
                (1, 2),
                (2, PAYOUT_STRUCTURE_DENOMINATOR - 2 - 1),
                (3, 1),
            ];
            // // ======= * it raises an error
            let result = az_trading_competition.competition_payout_structure_numerators_update(
                0,
                payout_structure_numerators.clone(),
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Numerator is greater than denominator.".to_string(),
                ))
            );
            // // ======= when new numerators causes the sum of numerators to be less than or equal to denominator
            payout_structure_numerators =
                vec![(0, 1), (1, 2), (2, PAYOUT_STRUCTURE_DENOMINATOR - 2 - 1)];
            // ======== when competition.payout_places is less than or equal to the highest position in payout_structure_numerators
            // ======== * it updates the payout_places to the highest position + 1
            az_trading_competition
                .competition_payout_structure_numerators_update(
                    0,
                    payout_structure_numerators.clone(),
                )
                .unwrap();
            competition = az_trading_competition.competitions.get(0).unwrap();
            assert_eq!(competition.payout_places, 3);
            // ======== * it saves the payout_structure_numerators
            assert_eq!(
                az_trading_competition
                    .competition_payout_structure_numerators
                    .get((0, payout_structure_numerators[0].0))
                    .unwrap(),
                payout_structure_numerators[0].1
            );
            assert_eq!(
                az_trading_competition
                    .competition_payout_structure_numerators
                    .get((0, payout_structure_numerators[1].0))
                    .unwrap(),
                payout_structure_numerators[1].1
            );
            assert_eq!(
                az_trading_competition
                    .competition_payout_structure_numerators
                    .get((0, payout_structure_numerators[2].0))
                    .unwrap(),
                payout_structure_numerators[2].1
            );
            // ======== * it updates the competition.payout_structure_numerator_sum
            assert_eq!(
                competition.payout_structure_numerator_sum,
                PAYOUT_STRUCTURE_DENOMINATOR
            );
            // ======== when competition.payout_places is greater than the highest position in payout_structure_numerators
            // ======== * it does not change competition.payout_places
            payout_structure_numerators.pop();
            az_trading_competition
                .competition_payout_structure_numerators_update(
                    0,
                    payout_structure_numerators.clone(),
                )
                .unwrap();
            assert_eq!(competition.payout_places, 3);
        }

        #[ink::test]
        fn test_register() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.register(0);
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

        #[ink::test]
        fn test_swap_exact_tokens_for_tokens() {
            let (accounts, mut az_trading_competition) = init();
            let id: u64 = 0;
            let mut amount_in: u128 = 555;
            let amount_out_min: u128 = 555;
            let mut path: Vec<AccountId> = vec![];
            let deadline: u64 = MOCK_START + MINIMUM_DURATION;
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.swap_exact_tokens_for_tokens(
                id,
                amount_in,
                amount_out_min,
                path.clone(),
                deadline,
            );
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
            // = when path is empty
            // = * it raises an error
            let result = az_trading_competition.swap_exact_tokens_for_tokens(
                id,
                amount_in,
                amount_out_min,
                path.clone(),
                deadline,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Path is empty.".to_string(),
                ))
            );
            // = when path is present
            path = vec![
                AccountId::try_from(*b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
                AccountId::try_from(*b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").unwrap(),
                AccountId::try_from(*b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap(),
            ];
            // == when competition hasn't started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START - 1);
            // = * it raises an error
            let result = az_trading_competition.swap_exact_tokens_for_tokens(
                id,
                amount_in,
                amount_out_min,
                path.clone(),
                deadline,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition isn't in progress.".to_string(),
                ))
            );
            // == when competition has ended
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                MOCK_START + MINIMUM_DURATION + 1,
            );
            // == * it raises an error
            let result = az_trading_competition.swap_exact_tokens_for_tokens(
                id,
                amount_in,
                amount_out_min,
                path.clone(),
                deadline,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition isn't in progress.".to_string(),
                ))
            );
            // == when competition is in progress
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                MOCK_START + MINIMUM_DURATION,
            );
            // === when user is not registered
            // === * it raises an error
            let result = az_trading_competition.swap_exact_tokens_for_tokens(
                id,
                amount_in,
                amount_out_min,
                path.clone(),
                deadline,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "You are not registered for this competition.".to_string(),
                ))
            );
            // === when user is registered
            az_trading_competition
                .competition_token_users
                .insert((0, mock_entry_fee_token(), accounts.bob), &0);
            // ==== when any of the tokens in path are invalid
            let result = az_trading_competition.swap_exact_tokens_for_tokens(
                id,
                amount_in,
                amount_out_min,
                path,
                deadline,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Path is invalid.".to_string(),
                ))
            );
            // ==== when path is valid
            path = vec![
                AccountId::try_from(*b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
                AccountId::try_from(*b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").unwrap(),
                AccountId::try_from(*b"tttttttttttttttttttttttttttttttt").unwrap(),
            ];
            // ===== when amount_in is greater than what is available to user
            amount_in = az_trading_competition
                .competition_token_users
                .get((id, path[0], accounts.bob))
                .unwrap_or(0)
                + 1;
            // ===== * it raises an error
            let result = az_trading_competition.swap_exact_tokens_for_tokens(
                id,
                amount_in,
                amount_out_min,
                path.clone(),
                deadline,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Insufficient balance.".to_string(),
                ))
            );
            // ===== when amount_in is available to user
            amount_in = az_trading_competition
                .competition_token_users
                .get((id, path[0], accounts.bob))
                .unwrap_or(0);
            // ====== when deadline is greater than competition end
            // ====== * it raises an error
            let result = az_trading_competition.swap_exact_tokens_for_tokens(
                id,
                amount_in,
                amount_out_min,
                path,
                deadline + 1,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Deadline is after competition end.".to_string(),
                ))
            );
            // ====== when deadline is <= competition.end
            // ====== THE REST NEEDS TO HAPPEN IN INTEGRATION TESTS
        }
    }
}
