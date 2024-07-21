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
    use primitive_types::U256;

    // === TYPES ===
    type Event = <AzTradingCompetition as ContractEventBase>::Type;
    type Result<T> = core::result::Result<T, AzTradingCompetitionError>;

    // === EVENTS ===
    #[ink(event)]
    pub struct CollectAdminFee {
        #[ink(topic)]
        id: u64,
    }

    #[ink(event)]
    pub struct CompetitionsCreate {
        #[ink(topic)]
        id: u64,
        start: Timestamp,
        end: Timestamp,
        entry_fee_token: AccountId,
        entry_fee_amount: Balance,
        admin_fee_percentage_numerator: u16,
        creator: AccountId,
    }

    #[ink(event)]
    pub struct CompetitionUserFinalValueUpdate {
        id: u64,
        user: AccountId,
        value: String,
    }

    #[ink(event)]
    pub struct Deregister {
        #[ink(topic)]
        id: u64,
        user: AccountId,
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
    // 10% of entry fee
    const DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR: u16 = 1_000;
    const DIA_USD_DECIMALS_FACTOR: Balance = 1_000_000_000_000_000_000;
    // Minimum 1 hour
    const MINIMUM_DURATION: Timestamp = 3_600_000;
    const PERCENTAGE_CALCULATION_DENOMINATOR: u16 = 10_000;
    const VALID_DIA_PRICE_SYMBOLS: &[&str] = &["AZERO/USD", "ETH/USD", "USDC/USD", "USDT/USD"];

    // === STRUCTS ===
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Config {
        pub admin: AccountId,
        pub allowed_pair_token_combinations_vec: Vec<(AccountId, AccountId)>,
        pub competitions_count: u64,
        pub default_admin_fee_percentage_numerator: u16,
        pub dia: AccountId,
        pub minimum_duration: Timestamp,
        pub percentage_calculation_denominator: u16,
        pub router: AccountId,
        pub token_dia_price_symbols_vec: Vec<(AccountId, String)>,
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
        pub admin_fee_collected: bool,
        pub admin_fee_percentage_numerator: u16,
        pub payout_places: u16,
        pub payout_structure_numerator_sum: u16,
        pub payout_winning_price_and_user_counts: Vec<(String, u32)>,
        pub token_prices_vec: Vec<(Timestamp, Balance)>,
        pub user_count: u32,
        pub user_final_value_updated_count: u32,
        pub creator: AccountId,
    }

    #[derive(scale::Decode, scale::Encode, Debug, Clone, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct CompetitionUser {
        pub final_value: Option<String>,
    }

    // === CONTRACT ===
    #[ink(storage)]
    pub struct AzTradingCompetition {
        allowed_pair_token_combinations_mapping: Mapping<AccountId, Vec<AccountId>>,
        allowed_pair_token_combinations_vec: Vec<(AccountId, AccountId)>,
        admin: AccountId,
        competition_payout_structure_numerators: Mapping<(u64, u16), u16>,
        competition_token_prices: Mapping<(u64, AccountId), Balance>,
        competition_token_users: Mapping<(u64, AccountId, AccountId), Balance>,
        competition_users: Mapping<(u64, AccountId), CompetitionUser>,
        competitions: Mapping<u64, Competition>,
        competitions_count: u64,
        dia: AccountId,
        dia_price_symbol_tokens_mapping: Mapping<String, AccountId>,
        router: AccountId,
        token_dia_price_symbols_mapping: Mapping<AccountId, String>,
        token_dia_price_symbols_vec: Vec<(AccountId, String)>,
    }
    impl AzTradingCompetition {
        #[ink(constructor)]
        pub fn new(
            allowed_pair_token_combinations_vec: Vec<(AccountId, AccountId)>,
            dia: AccountId,
            router: AccountId,
            token_dia_price_symbols_vec: Vec<(AccountId, String)>,
        ) -> Result<Self> {
            let mut x = Self {
                admin: Self::env().caller(),
                allowed_pair_token_combinations_mapping: Mapping::default(),
                allowed_pair_token_combinations_vec: allowed_pair_token_combinations_vec.clone(),
                competition_payout_structure_numerators: Mapping::default(),
                competition_token_prices: Mapping::default(),
                competition_token_users: Mapping::default(),
                competition_users: Mapping::default(),
                competitions: Mapping::default(),
                competitions_count: 0,
                dia,
                dia_price_symbol_tokens_mapping: Mapping::default(),
                router,
                token_dia_price_symbols_mapping: Mapping::default(),
                token_dia_price_symbols_vec: token_dia_price_symbols_vec.clone(),
            };
            for token_dia_price_symbol in token_dia_price_symbols_vec.iter() {
                if VALID_DIA_PRICE_SYMBOLS.contains(&&token_dia_price_symbol.1[..]) {
                    x.token_dia_price_symbols_mapping
                        .insert(token_dia_price_symbol.0, &token_dia_price_symbol.1);
                    x.dia_price_symbol_tokens_mapping
                        .insert(token_dia_price_symbol.1.clone(), &token_dia_price_symbol.0);
                } else {
                    return Err(AzTradingCompetitionError::UnprocessableEntity(
                        "Invalid DIA price symbol.".to_string(),
                    ));
                }
            }
            for allowed_pair_token_combination in allowed_pair_token_combinations_vec.iter() {
                if x.token_dia_price_symbols_mapping
                    .get(allowed_pair_token_combination.0)
                    .is_none()
                    || x.token_dia_price_symbols_mapping
                        .get(allowed_pair_token_combination.1)
                        .is_none()
                {
                    return Err(AzTradingCompetitionError::UnprocessableEntity(
                        "Invalid pair token combinations.".to_string(),
                    ));
                } else {
                    if let Some(mut allowed_to_tokens) = x
                        .allowed_pair_token_combinations_mapping
                        .get(allowed_pair_token_combination.0)
                    {
                        allowed_to_tokens.push(allowed_pair_token_combination.1);
                        x.allowed_pair_token_combinations_mapping
                            .insert(allowed_pair_token_combination.0, &allowed_to_tokens);
                    } else {
                        x.allowed_pair_token_combinations_mapping.insert(
                            allowed_pair_token_combination.0,
                            &vec![allowed_pair_token_combination.1],
                        );
                    }
                    if let Some(mut allowed_to_tokens) = x
                        .allowed_pair_token_combinations_mapping
                        .get(allowed_pair_token_combination.1)
                    {
                        allowed_to_tokens.push(allowed_pair_token_combination.0);
                        x.allowed_pair_token_combinations_mapping
                            .insert(allowed_pair_token_combination.1, &allowed_to_tokens);
                    } else {
                        x.allowed_pair_token_combinations_mapping.insert(
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
        pub fn competitions_show(&self, id: u64) -> Result<Competition> {
            self.competitions
                .get(id)
                .ok_or(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
        }

        #[ink(message)]
        pub fn competition_token_user(
            &self,
            id: u64,
            token: AccountId,
            user: AccountId,
        ) -> Result<Balance> {
            self.competition_token_users.get((id, token, user)).ok_or(
                AzTradingCompetitionError::NotFound("CompetitionTokenUser".to_string()),
            )
        }

        #[ink(message)]
        pub fn competition_users_show(&self, id: u64, user: AccountId) -> Result<CompetitionUser> {
            self.competition_users
                .get((id, user))
                .ok_or(AzTradingCompetitionError::NotFound(
                    "CompetitionUser".to_string(),
                ))
        }

        #[ink(message)]
        pub fn config(&self) -> Config {
            Config {
                admin: self.admin,
                allowed_pair_token_combinations_vec: self
                    .allowed_pair_token_combinations_vec
                    .clone(),
                competitions_count: self.competitions_count,
                default_admin_fee_percentage_numerator: DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR,
                dia: self.dia,
                minimum_duration: MINIMUM_DURATION,
                percentage_calculation_denominator: PERCENTAGE_CALCULATION_DENOMINATOR,
                router: self.router,
                token_dia_price_symbols_vec: self.token_dia_price_symbols_vec.clone(),
            }
        }

        #[ink(message)]
        pub fn get_latest_prices_from_dia(&self) -> Vec<Option<(Timestamp, Balance)>> {
            let dia_price_symbols_as_strings: Vec<String> = VALID_DIA_PRICE_SYMBOLS
                .iter()
                .map(|w| w.to_string())
                .collect::<Vec<String>>();
            build_call::<Environment>()
                .call(self.dia)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("get_latest_prices")))
                        .push_arg(dia_price_symbols_as_strings),
                )
                .returns::<Result<Vec<Option<(u64, u128)>>>>()
                .invoke()
                .unwrap()
        }

        // === HANDLES ===
        #[ink(message)]
        pub fn competitions_create(
            &mut self,
            start: Timestamp,
            end: Timestamp,
            entry_fee_token: AccountId,
            entry_fee_amount: Balance,
            admin_fee_percentage_numerator: Option<u16>,
        ) -> Result<Competition> {
            let caller: AccountId = Self::env().caller();
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
            if self
                .token_dia_price_symbols_mapping
                .get(entry_fee_token)
                .is_none()
            {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Entry fee token is not permitted.".to_string(),
                ));
            }
            let mut competition_admin_fee_percentage_numerator: u16 =
                DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR;
            if let Some(admin_fee_percentage_numerator_unwrapped) = admin_fee_percentage_numerator {
                if caller == self.admin {
                    if admin_fee_percentage_numerator_unwrapped
                        < DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR
                    {
                        competition_admin_fee_percentage_numerator =
                            admin_fee_percentage_numerator_unwrapped
                    } else {
                        return Err(AzTradingCompetitionError::UnprocessableEntity(
                            "Fee percentage numerator must be less than the default.".to_string(),
                        ));
                    }
                } else {
                    return Err(AzTradingCompetitionError::Unauthorised);
                }
            }

            let competition: Competition = Competition {
                id: self.competitions_count,
                start,
                end,
                entry_fee_token,
                entry_fee_amount,
                admin_fee_collected: false,
                admin_fee_percentage_numerator: competition_admin_fee_percentage_numerator,
                payout_places: 0,
                payout_structure_numerator_sum: 0,
                payout_winning_price_and_user_counts: vec![],
                creator: caller,
                token_prices_vec: vec![],
                user_count: 0,
                user_final_value_updated_count: 0,
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
                    admin_fee_percentage_numerator: competition_admin_fee_percentage_numerator,
                    creator: caller,
                }),
            );

            Ok(competition)
        }

        // This needs review
        #[ink(message)]
        pub fn competition_payout_structure_numerators_update(
            &mut self,
            id: u64,
            payout_structure_numerators: Vec<(u16, u16)>,
        ) -> Result<u16> {
            let caller: AccountId = Self::env().caller();
            let mut competition: Competition = self.competitions_show(id)?;
            Self::authorise(competition.creator, caller)?;
            self.validate_competition_has_not_started(competition.start)?;
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
            if competition.payout_structure_numerator_sum > PERCENTAGE_CALCULATION_DENOMINATOR {
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

        // Should this have an option to do individual tokens?
        #[ink(message)]
        pub fn competition_token_prices_update(&mut self, id: u64) -> Result<()> {
            let mut competition: Competition = self.competitions_show(id)?;
            self.validate_competition_has_ended(competition.clone())?;
            // Validate that prices haven't been retrieved already
            if !competition.token_prices_vec.is_empty() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Token prices for competition already set.".to_string(),
                ));
            }

            let prices: Vec<Option<(Timestamp, Balance)>> = self.get_latest_prices_from_dia();
            for (index, price_details) in prices.iter().enumerate() {
                if let Some(price_details_unwrapped) = price_details {
                    competition.token_prices_vec.push(*price_details_unwrapped);
                    let price_symbol: String = VALID_DIA_PRICE_SYMBOLS[index].to_string();
                    let token: AccountId = self
                        .dia_price_symbol_tokens_mapping
                        .get(price_symbol)
                        .unwrap();
                    self.competition_token_prices
                        .insert((id, token), &price_details_unwrapped.1);
                } else {
                    return Err(AzTradingCompetitionError::UnprocessableEntity(
                        "Price details from DIA unavailable.".to_string(),
                    ));
                }
            }
            self.competitions.insert(id, &competition);

            Ok(())
        }

        #[ink(message)]
        pub fn collect_competition_admin_fee(&mut self, id: u64) -> Result<Balance> {
            // 1. Validate caller is admin
            let caller: AccountId = Self::env().caller();
            Self::authorise(self.admin, caller)?;
            // 2. Get competition
            let mut competition: Competition = self.competitions_show(id)?;
            // 3. Validate that competition has started
            self.validate_competition_has_started(competition.start)?;
            // 4. Validate that user count is greater than or equal to payout_places
            if competition.user_count < competition.payout_places.into() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition hasn't met minimum user requirements.".to_string(),
                ));
            }
            // 5. Validate that admin fee hasn't been collected yet
            if competition.admin_fee_collected {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Admin fee has already been colleted.".to_string(),
                ));
            }
            // 6. Transfer admin fee to admin
            let admin_fee: Balance = Balance::from(competition.user_count)
                * (U256::from(competition.entry_fee_amount)
                    * U256::from(competition.admin_fee_percentage_numerator)
                    / U256::from(DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR))
                .as_u128();
            PSP22Ref::transfer_builder(&competition.entry_fee_token, caller, admin_fee, vec![])
                .call_flags(CallFlags::default())
                .invoke()?;
            // 7. Update competition.admin_fee_collected
            competition.admin_fee_collected = true;
            self.competitions.insert(id, &competition);

            // emit event
            Self::emit_event(self.env(), Event::CollectAdminFee(CollectAdminFee { id }));

            Ok(admin_fee)
        }

        // This isn't the final USD value as it doesn't factor in each token's decimal points.
        // Doesn't matter though as it can still be used to find out who the winners are.
        #[ink(message)]
        pub fn competition_user_final_value_update(
            &mut self,
            id: u64,
            user: AccountId,
        ) -> Result<String> {
            // 1. Get competition
            let mut competition: Competition = self.competitions_show(id)?;
            // 2. Validate competition has ended
            self.validate_competition_has_ended(competition.clone())?;
            // 3. Get CompetitionUser
            let mut competition_user: CompetitionUser = self.competition_users_show(id, user)?;
            // 4. Validate CompetitionUser hasn't been processed
            if competition_user.final_value.is_some() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "CompetitionUser already processed.".to_string(),
                ));
            }
            // 5. Validate competition token prices have been set
            if competition.token_prices_vec.is_empty() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Token prices haven't been set.".to_string(),
                ));
            }

            // 6. Calculate usd value
            let mut user_value: U256 = U256::from(0);
            for dia_price_symbol in VALID_DIA_PRICE_SYMBOLS.iter() {
                let token: AccountId = self
                    .dia_price_symbol_tokens_mapping
                    .get(dia_price_symbol.to_string())
                    .unwrap();
                let price: Balance = self
                    .competition_token_prices
                    .get((competition.id, token))
                    .unwrap();
                let token_balance: Balance = self
                    .competition_token_users
                    .get((id, token, user))
                    .unwrap_or(0);
                user_value += U256::from(price) * U256::from(token_balance)
            }
            // 6. Set final_value
            let user_value_as_string: String = user_value.to_string();
            competition_user.final_value = Some(user_value_as_string.clone());
            self.competition_users.insert((id, user), &competition_user);
            // 7. Increase competition.user_final_value_updated_count
            competition.user_final_value_updated_count += 1;
            self.competitions.insert(competition.id, &competition);

            // emit event
            Self::emit_event(
                self.env(),
                Event::CompetitionUserFinalValueUpdate(CompetitionUserFinalValueUpdate {
                    id: competition.id,
                    user,
                    value: user_value_as_string.clone(),
                }),
            );

            Ok(user_value_as_string)
        }

        #[ink(message)]
        pub fn deregister(&mut self, id: u64) -> Result<()> {
            // 1. Get competition
            let mut competition: Competition = self.competitions_show(id)?;
            // 2. Validate that user is registered
            let caller: AccountId = Self::env().caller();
            let competition_token_user: Balance =
                self.competition_token_user(id, competition.entry_fee_token, caller)?;
            // 3. Validate able to deregister
            if Self::env().block_timestamp() >= competition.start
                && competition.user_count >= competition.payout_places.into()
            {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Unable to deregister when competition has started and minimum user requirements met.".to_string(),
                ));
            }

            // 4. Transfer token back to user
            PSP22Ref::transfer_builder(
                &competition.entry_fee_token,
                caller,
                competition_token_user,
                vec![],
            )
            .call_flags(CallFlags::default())
            .invoke()?;
            // 5. Remove competition token user
            self.competition_token_users
                .remove((id, competition.entry_fee_token, caller));
            // 6. Update competition
            competition.user_count -= 1;
            self.competitions.insert(id, &competition);

            // emit event
            Self::emit_event(
                self.env(),
                Event::Deregister(Deregister { id, user: caller }),
            );

            Ok(())
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
            // 1. Validate that numerator is equal to denominator
            if competition.payout_structure_numerator_sum != PERCENTAGE_CALCULATION_DENOMINATOR {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Payout structure is not set yet.".to_string(),
                ));
            }
            // 2. Validate that time is before start
            self.validate_competition_has_not_started(competition.start)?;
            // 3. Validate that user hasn't registered already
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

            // 4. Acquire token from caller
            self.acquire_psp22(
                competition.entry_fee_token,
                caller,
                competition.entry_fee_amount,
            )?;
            // 5. Figure out admin fee
            let admin_fee: Balance = (U256::from(competition.entry_fee_amount)
                * U256::from(competition.admin_fee_percentage_numerator)
                / U256::from(DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR))
            .as_u128();
            // 6. Set balance of competition token user
            self.competition_token_users.insert(
                (id, competition.entry_fee_token, caller),
                &(competition.entry_fee_amount - admin_fee),
            );
            // 7. Increase competition.user_count
            competition.user_count += 1;
            self.competitions.insert(competition.id, &competition);
            // 8. Create CompetitionUser
            self.competition_users.insert(
                (competition.id, caller),
                &CompetitionUser { final_value: None },
            );

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
            // 1. Validate that there's enough users in competition
            if competition.user_count < competition.payout_places.into() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition is invalid, please deregister.".to_string(),
                ));
            }
            // 2. Validate that competition is in progress
            self.validate_competition_is_in_progress(competition.clone())?;
            // 3. Validate that user has enough to cover amount_in
            let caller: AccountId = Self::env().caller();
            let in_balance: Balance =
                self.competition_token_user(id, competition.entry_fee_token, caller)?;
            if amount_in > in_balance {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Insufficient balance.".to_string(),
                ));
            }
            // 4. Validate that path is valid
            let mut previous_token: Option<AccountId> = None;
            for token in path.iter() {
                if previous_token.is_some() {
                    let mut valid = false;
                    if let Some(to_tokens) = self
                        .allowed_pair_token_combinations_mapping
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
            self.competition_token_users
                .insert((id, in_token, caller), &(in_balance - amount_in));
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

        fn emit_event<EE: EmitEvent<Self>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }

        fn validate_competition_has_ended(&self, competition: Competition) -> Result<()> {
            if Self::env().block_timestamp() <= competition.end {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition hasn't ended.".to_string(),
                ));
            }

            Ok(())
        }

        fn validate_competition_has_not_started(&self, start: Timestamp) -> Result<()> {
            if Self::env().block_timestamp() >= start {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition has started".to_string(),
                ));
            }

            Ok(())
        }

        fn validate_competition_has_started(&self, start: Timestamp) -> Result<()> {
            if Self::env().block_timestamp() < start {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition hasn't started".to_string(),
                ));
            }

            Ok(())
        }

        fn validate_competition_is_in_progress(&self, competition: Competition) -> Result<()> {
            if Self::env().block_timestamp() < competition.start
                || Self::env().block_timestamp() > competition.end
            {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition isn't in progress.".to_string(),
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
                mock_allowed_pair_token_combinations(),
                mock_dia_address(),
                mock_router_address(),
                mock_token_to_dia_price_symbol_combos(),
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

        fn mock_dia_address() -> AccountId {
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
            assert_eq!(
                config.allowed_pair_token_combinations_vec,
                mock_allowed_pair_token_combinations()
            );
            assert_eq!(
                config.default_admin_fee_percentage_numerator,
                DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR
            );
            assert_eq!(config.dia, mock_dia_address());
            assert_eq!(config.minimum_duration, MINIMUM_DURATION);
            assert_eq!(
                config.percentage_calculation_denominator,
                PERCENTAGE_CALCULATION_DENOMINATOR
            );
            assert_eq!(config.router, az_trading_competition.router);
            assert_eq!(
                config.token_dia_price_symbols_vec,
                mock_token_to_dia_price_symbol_combos()
            );
        }

        // === TEST HANDLES ===
        #[ink::test]
        fn test_collect_competition_admin_fee() {
            let (accounts, mut az_trading_competition) = init();
            // when called by non-admin
            set_caller::<DefaultEnvironment>(accounts.charlie);
            let result = az_trading_competition.collect_competition_admin_fee(0);
            assert_eq!(result, Err(AzTradingCompetitionError::Unauthorised));
            // when called by admin
            set_caller::<DefaultEnvironment>(accounts.bob);
            // = when competition does not exist
            // = * it raises an error
            let result = az_trading_competition.collect_competition_admin_fee(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
            );
            // = when competition exists
            az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                    None,
                )
                .unwrap();
            // == when competition hasn't started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START - 1);
            // == * it raises an error
            let result = az_trading_competition.collect_competition_admin_fee(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition hasn't started".to_string(),
                ))
            );
            // == when competition has started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START);
            // === when competition has not met minimum user requirements
            let mut competition = az_trading_competition.competitions.get(0).unwrap();
            competition.payout_places = 1;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // === * it raises an error
            let result = az_trading_competition.collect_competition_admin_fee(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition hasn't met minimum user requirements.".to_string(),
                ))
            );
            // === when competition has met minimum user requirements
            competition.user_count = (competition.payout_places + 1).into();
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // ==== when competition admin fee has already been collected
            competition.admin_fee_collected = true;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // ==== * it raises an error
            let result = az_trading_competition.collect_competition_admin_fee(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Admin fee has already been colleted.".to_string(),
                ))
            );
            // ==== when competition admin fee hasn't been collected
            // ==== NEED TO DO IN INTEGRATION TEST
        }

        #[ink::test]
        fn test_competitions_create() {
            let (accounts, mut az_trading_competition) = init();
            // when competitions_count is u64 max
            az_trading_competition.competitions_count = u64::MAX;
            // * it raises an error
            let result = az_trading_competition.competitions_create(
                MOCK_START,
                MOCK_END,
                mock_entry_fee_token(),
                MOCK_ENTRY_FEE_AMOUNT,
                None,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Max number of competitions reached.".to_string(),
                ))
            );
            // when competitions_count is less than u64 max
            az_trading_competition.competitions_count = u64::MAX - 2;
            // = when duration is less than or equal to MINIMUM_DURATION
            let result = az_trading_competition.competitions_create(
                MOCK_START,
                MOCK_START + MINIMUM_DURATION - 1,
                mock_entry_fee_token(),
                MOCK_ENTRY_FEE_AMOUNT,
                None,
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
                None,
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
                mock_dia_address(),
                MOCK_ENTRY_FEE_AMOUNT,
                None,
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
                    None,
                )
                .unwrap();
            // ==== when admin_fee_percentage_numerator is not present
            // ==== * it stores the competition with default fee percentage numerator
            assert_eq!(
                az_trading_competition
                    .competitions
                    .get(&competitions_count)
                    .unwrap()
                    .admin_fee_percentage_numerator,
                DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR
            );
            // ==== * it increases the competitions_count by 1
            assert_eq!(
                az_trading_competition.competitions_count,
                competitions_count + 1
            );
            // ==== when fee_percentage_numerator is present
            let mut admin_fee_percentage_numerator: Option<u16> =
                Some(DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR);
            // ===== when called by non-admin
            set_caller::<DefaultEnvironment>(accounts.charlie);
            // ===== * it raises an error
            let result = az_trading_competition.competitions_create(
                MOCK_START,
                MOCK_START + MINIMUM_DURATION,
                mock_entry_fee_token(),
                MOCK_ENTRY_FEE_AMOUNT,
                admin_fee_percentage_numerator,
            );
            assert_eq!(result, Err(AzTradingCompetitionError::Unauthorised));
            // ===== when called by admin
            set_caller::<DefaultEnvironment>(accounts.bob);
            // ====== when admin_fee_percentage_numerator is greater than or equal to default_fee_percentage_numerate
            // ====== * it raises an error
            let result = az_trading_competition.competitions_create(
                MOCK_START,
                MOCK_START + MINIMUM_DURATION,
                mock_entry_fee_token(),
                MOCK_ENTRY_FEE_AMOUNT,
                admin_fee_percentage_numerator,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Fee percentage numerator must be less than the default.".to_string(),
                ))
            );
            // ====== when admin_fee_percentage_numerator is less than default_fee_percentage_numerate
            admin_fee_percentage_numerator = Some(DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR - 1);
            // ======= * it stores the competition with provided fee percentage numerator
            az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                    admin_fee_percentage_numerator,
                )
                .unwrap();
            assert_eq!(
                az_trading_competition
                    .competitions
                    .get(&competitions_count + 1)
                    .unwrap()
                    .admin_fee_percentage_numerator,
                admin_fee_percentage_numerator.unwrap(),
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
                    None,
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
            // ===== when a numerator is zero
            payout_structure_numerators = vec![
                (0, 1),
                (1, 2),
                (2, PERCENTAGE_CALCULATION_DENOMINATOR - 2 - 1),
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
                (2, PERCENTAGE_CALCULATION_DENOMINATOR - 2 - 1),
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
            payout_structure_numerators = vec![
                (0, 1),
                (1, 2),
                (2, PERCENTAGE_CALCULATION_DENOMINATOR - 2 - 1),
            ];
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
                PERCENTAGE_CALCULATION_DENOMINATOR
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
        fn test_competition_token_prices_update() {
            let (_accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.competition_token_prices_update(0);
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
                    None,
                )
                .unwrap();
            // = when competition has not ended
            // = * it raises an error
            let result = az_trading_competition.competition_token_prices_update(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition hasn't ended.".to_string(),
                ))
            );
            // = when competition has ended
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                MOCK_START + MINIMUM_DURATION + 1,
            );
            // == when final prices have already been recorded
            let mut competition: Competition = az_trading_competition.competitions.get(0).unwrap();
            competition.token_prices_vec = vec![(5, 5)];
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            let result = az_trading_competition.competition_token_prices_update(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Token prices for competition already set.".to_string(),
                ))
            );
        }

        #[ink::test]
        fn test_deregister() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.deregister(0);
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
                    None,
                )
                .unwrap();
            // = when user is not registered
            // = * it raises an error
            let result = az_trading_competition.deregister(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "CompetitionTokenUser".to_string(),
                ))
            );
            // = when user is registered
            az_trading_competition.competition_token_users.insert(
                (0, mock_entry_fee_token(), accounts.bob),
                &MOCK_ENTRY_FEE_AMOUNT,
            );
            // == when competition has started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START);
            // === when user count is equal to or greater than payout places
            // === * it raises an error
            let result = az_trading_competition.deregister(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Unable to deregister when competition has started and minimum user requirements met.".to_string(),
                ))
            );
            // == NEEDS TO BE DONE IN INTEGRATION TESTS
            // === when user count is less than the amount of payout places
            // == when competition hasn't started
            // == * it sends the entry fee back to user
            // == * it removes competition token user
            // == * it decreases the competition user count
        }

        #[ink::test]
        fn test_final_value_update() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result =
                az_trading_competition.competition_user_final_value_update(0, accounts.bob);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
            );
            // when competition exists
            let mut competition: Competition = az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                    None,
                )
                .unwrap();
            // = when competition hasn't ended
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(competition.end);
            // = * it raises an error
            let result =
                az_trading_competition.competition_user_final_value_update(0, accounts.bob);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition hasn't ended.".to_string(),
                ))
            );
            // = when competition has ended
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                competition.end + 1,
            );
            // == when CompetitionUser doesn't exist
            let result =
                az_trading_competition.competition_user_final_value_update(0, accounts.bob);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "CompetitionUser".to_string(),
                ))
            );
            // == when CompetitionUser exists
            let mut competition_user: CompetitionUser = CompetitionUser {
                final_value: Some(0.to_string()),
            };
            az_trading_competition
                .competition_users
                .insert((0, accounts.bob), &competition_user);
            // === when CompetitionUser is processed already
            // === * it raises an error
            let result =
                az_trading_competition.competition_user_final_value_update(0, accounts.bob);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "CompetitionUser already processed.".to_string(),
                ))
            );
            // === when CompetitionUser doesn't have final_value
            competition_user.final_value = None;
            az_trading_competition
                .competition_users
                .insert((0, accounts.bob), &competition_user);
            // ==== when competion token prices haven't been set
            // ==== * it raises an error
            let result =
                az_trading_competition.competition_user_final_value_update(0, accounts.bob);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Token prices haven't been set.".to_string(),
                ))
            );
            // ==== when competion token prices have been set
            competition.token_prices_vec = [
                (1721529505000, 422649090041300300),
                (1721531044000, 3514376553083345700000),
                (1721489651000, 1000078250788530200),
                (1721480044001, 1000479999999999000),
            ]
            .to_vec();
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            let mut usd_usd_value: Balance = 0;
            for (index, mock_token_to_dia_price_symbol_combo) in
                mock_token_to_dia_price_symbol_combos().iter().enumerate()
            {
                az_trading_competition.competition_token_prices.insert(
                    (competition.id, mock_token_to_dia_price_symbol_combo.0),
                    &competition.token_prices_vec[index].1,
                );
                az_trading_competition.competition_token_users.insert(
                    (
                        competition.id,
                        mock_token_to_dia_price_symbol_combo.0,
                        accounts.bob,
                    ),
                    &1,
                );
                usd_usd_value += competition.token_prices_vec[index].1
            }
            az_trading_competition
                .competition_user_final_value_update(0, accounts.bob)
                .unwrap();
            // ==== * it sets the final_value for the user
            let final_value: String = az_trading_competition
                .competition_users
                .get((competition.id, accounts.bob))
                .unwrap()
                .final_value
                .unwrap();
            assert_eq!(final_value, usd_usd_value.to_string());
            // ==== * it increases the competition.user_final_value_updated_count by one
            competition = az_trading_competition
                .competitions
                .get(competition.id)
                .unwrap();
            assert_eq!(competition.user_final_value_updated_count, 1)
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
                    None,
                )
                .unwrap();
            // = when competition numerator does not equal denominator
            // = * it raises an error
            let result = az_trading_competition.register(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Payout structure is not set yet.".to_string(),
                ))
            );
            // = when competition numerator equals denominator
            let mut competition: Competition = az_trading_competition.competitions_show(0).unwrap();
            competition.payout_structure_numerator_sum = PERCENTAGE_CALCULATION_DENOMINATOR;
            az_trading_competition.competitions.insert(0, &competition);
            // == when competition has started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START);
            // * it raises an error
            let result = az_trading_competition.register(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition has started".to_string(),
                ))
            );
            // == when competition has not started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START - 1);
            // == when user has registered already
            az_trading_competition.competition_token_users.insert(
                (0, mock_entry_fee_token(), accounts.bob),
                &MOCK_ENTRY_FEE_AMOUNT,
            );
            // == * it raises an error
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
                    None,
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
            // == when competition user count is less than payout places
            let mut competition: Competition = az_trading_competition.competitions_show(0).unwrap();
            competition.payout_places = 1;
            az_trading_competition.competitions.insert(0, &competition);
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
                    "Competition is invalid, please deregister.".to_string(),
                ))
            );
            // == when competition user count is greater than or equal to payout places
            competition.user_count = competition.payout_places.into();
            az_trading_competition.competitions.insert(0, &competition);
            // === when competition hasn't started
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
            // === when competition has ended
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                MOCK_START + MINIMUM_DURATION + 1,
            );
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
                    "Competition isn't in progress.".to_string(),
                ))
            );
            // === when competition is in progress
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                MOCK_START + MINIMUM_DURATION,
            );
            // ==== when competition trading user is not present
            // ==== * it raises an error
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
                    "CompetitionTokenUser".to_string(),
                ))
            );
            // ==== when competition trading user is present
            az_trading_competition
                .competition_token_users
                .insert((0, mock_entry_fee_token(), accounts.bob), &0);
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
            // ====== when any of the tokens in path are invalid
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
            // ====== when path is valid
            path = vec![
                AccountId::try_from(*b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
                AccountId::try_from(*b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").unwrap(),
                AccountId::try_from(*b"tttttttttttttttttttttttttttttttt").unwrap(),
            ];
            // ======= when deadline is greater than competition end
            // ======= * it raises an error
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
            // ======= when deadline is <= competition.end
            // ======= THE REST NEEDS TO HAPPEN IN INTEGRATION TESTS
        }
    }
}
