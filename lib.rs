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
    pub struct CollectPrize {
        #[ink(topic)]
        id: u64,
        #[ink(topic)]
        competitor: AccountId,
        #[ink(topic)]
        token: AccountId,
        amount: Balance,
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
        azero_processing_fee: Balance,
        creator: AccountId,
    }

    #[ink(event)]
    pub struct CompetitorFinalValueUpdate {
        id: u64,
        competitor: AccountId,
        value: String,
    }

    #[ink(event)]
    pub struct Deregister {
        #[ink(topic)]
        id: u64,
        competitor: AccountId,
    }

    #[ink(event)]
    pub struct JudgeUpdate {
        #[ink(topic)]
        id: u64,
        #[ink(topic)]
        judge: AccountId,
    }

    #[ink(event)]
    pub struct NextJudgeUpdate {
        #[ink(topic)]
        id: u64,
        #[ink(topic)]
        user: AccountId,
    }

    #[ink(event)]
    pub struct PayoutStructureUpdate {
        #[ink(topic)]
        id: u64,
        payout_structure_numerators: Vec<(u16, u16)>,
    }

    #[ink(event)]
    pub struct PlaceCompetitor {
        #[ink(topic)]
        id: u64,
        competitors_addresses: Vec<AccountId>,
    }

    #[ink(event)]
    pub struct Register {
        #[ink(topic)]
        id: u64,
        competitor: AccountId,
    }

    #[ink(event)]
    pub struct Swap {
        id: u64,
        competitor: AccountId,
        in_token: AccountId,
        in_amount: Balance,
        out_token: AccountId,
        out_amount: Balance,
    }

    // === CONSTANTS ===
    const DAY_IN_MS: Timestamp = 86_400_000;
    // 10% of entry fee
    const DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR: u16 = 1_000;
    const DIA_USD_DECIMALS_FACTOR: Balance = 1_000_000_000_000_000_000;
    // Minimum 1 hour
    const MINIMUM_DURATION: Timestamp = 3_600_000;
    const PERCENTAGE_CALCULATION_DENOMINATOR: u16 = 10_000;
    const FINAL_VALUE_UPDATE_FEE_PERCENTAGE_NUMERATOR: u16 = 1_000;
    const VALID_DIA_PRICE_SYMBOLS: &[&str] = &["AZERO/USD", "ETH/USD", "USDC/USD", "USDT/USD"];

    // === STRUCTS ===
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Config {
        pub admin: AccountId,
        pub allowed_pair_token_combinations_vec: Vec<(AccountId, AccountId)>,
        pub competitions_count: u64,
        pub default_admin_fee_percentage_numerator: u16,
        pub default_azero_processing_fee: Balance,
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
        pub azero_processing_fee: Balance,
        pub judge: AccountId,
        pub judge_place_attempt: u128,
        pub next_judge: Option<AccountId>,
        pub payout_places: u16,
        pub payout_structure_numerator_sum: u16,
        pub token_prices_vec: Vec<(Timestamp, Balance)>,
        pub competitors_count: u32,
        pub competitor_final_value_updated_count: u32,
        pub competitors_placed_count: u32,
        pub creator: AccountId,
    }

    #[derive(scale::Decode, scale::Encode, Debug, Clone, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct CompetitionJudge {
        pub deadline: Timestamp,
    }

    #[derive(scale::Decode, scale::Encode, Debug, Clone, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct CompetitionTokenCompetitor {
        pub amount: Balance,
        pub collected: bool,
    }

    #[derive(scale::Decode, scale::Encode, Debug, Clone, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct CompetitionTokenPrize {
        pub amount: Balance,
        pub collected: Balance,
    }

    #[derive(scale::Decode, scale::Encode, Debug, Clone, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Competitor {
        pub final_value: Option<String>,
        pub judge_place_attempt: u128,
        pub competition_place_details_index: u32,
    }

    #[derive(scale::Decode, scale::Encode, Debug, Clone, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct CompetitionPlaceDetail {
        pub competitor_value: String,
        pub competitors_count: u32,
        pub payout_numerator: u16,
    }

    // === CONTRACT ===
    #[ink(storage)]
    pub struct AzTradingCompetition {
        allowed_pair_token_combinations_mapping: Mapping<AccountId, Vec<AccountId>>,
        allowed_pair_token_combinations_vec: Vec<(AccountId, AccountId)>,
        admin: AccountId,
        competition_judges: Mapping<(u64, AccountId), CompetitionJudge>,
        competition_payout_structure_numerators: Mapping<(u64, u16), u16>,
        // Ordered- by competitor final value
        competition_place_details: Mapping<u64, Vec<CompetitionPlaceDetail>>,
        competition_token_prices: Mapping<(u64, AccountId), Balance>,
        competition_token_prizes: Mapping<(u64, AccountId), CompetitionTokenPrize>,
        competition_token_competitors:
            Mapping<(u64, AccountId, AccountId), CompetitionTokenCompetitor>,
        competitors: Mapping<(u64, AccountId), Competitor>,
        competitions: Mapping<u64, Competition>,
        competitions_count: u64,
        default_azero_processing_fee: Balance,
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
            default_azero_processing_fee: Balance,
            dia: AccountId,
            router: AccountId,
            token_dia_price_symbols_vec: Vec<(AccountId, String)>,
        ) -> Result<Self> {
            let mut x = Self {
                admin: Self::env().caller(),
                allowed_pair_token_combinations_mapping: Mapping::default(),
                allowed_pair_token_combinations_vec: allowed_pair_token_combinations_vec.clone(),
                competition_judges: Mapping::default(),
                competition_payout_structure_numerators: Mapping::default(),
                competition_place_details: Mapping::default(),
                competition_token_prices: Mapping::default(),
                competition_token_prizes: Mapping::default(),
                competition_token_competitors: Mapping::default(),
                competitors: Mapping::default(),
                competitions: Mapping::default(),
                competitions_count: 0,
                default_azero_processing_fee,
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
        pub fn competition_place_details_show(
            &self,
            id: u64,
            index: u32,
        ) -> Result<CompetitionPlaceDetail> {
            let competition_place_details_vec: Vec<CompetitionPlaceDetail> =
                self.competition_place_details.get(id).ok_or(
                    AzTradingCompetitionError::NotFound("CompetitionPlaceDetail".to_string()),
                )?;
            if index >= competition_place_details_vec.len().try_into().unwrap() {
                return Err(AzTradingCompetitionError::NotFound(
                    "CompetitionPlaceDetail".to_string(),
                ));
            }

            Ok(competition_place_details_vec[usize::try_from(index).unwrap()].clone())
        }

        #[ink(message)]
        pub fn competition_token_competitors_show(
            &self,
            id: u64,
            token: AccountId,
            competitor_address: AccountId,
        ) -> Result<CompetitionTokenCompetitor> {
            self.competition_token_competitors
                .get((id, token, competitor_address))
                .ok_or(AzTradingCompetitionError::NotFound(
                    "CompetitionTokenCompetitor".to_string(),
                ))
        }

        #[ink(message)]
        pub fn competition_token_prizes_show(
            &self,
            id: u64,
            token: AccountId,
        ) -> Result<CompetitionTokenPrize> {
            self.competition_token_prizes.get((id, token)).ok_or(
                AzTradingCompetitionError::NotFound("CompetitionTokenPrize".to_string()),
            )
        }

        #[ink(message)]
        pub fn competitors_show(
            &self,
            id: u64,
            competitor_address: AccountId,
        ) -> Result<Competitor> {
            self.competitors.get((id, competitor_address)).ok_or(
                AzTradingCompetitionError::NotFound("Competitor".to_string()),
            )
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
                default_azero_processing_fee: self.default_azero_processing_fee,
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
        pub fn collect_competition_admin_fee(&mut self, id: u64) -> Result<Balance> {
            // 1. Validate caller is admin
            let caller: AccountId = Self::env().caller();
            Self::authorise(self.admin, caller)?;
            // 2. Get competition
            let mut competition: Competition = self.competitions_show(id)?;
            // 3. Validate that competition has started
            self.validate_competition_has_started(competition.start)?;
            // 4. Validate that competitor count is greater than or equal to payout_places
            if competition.competitors_count < competition.payout_places.into() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition hasn't met minimum competitor requirements.".to_string(),
                ));
            }
            // 5. Validate that admin fee hasn't been collected yet
            if competition.admin_fee_collected {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Admin fee has already been colleted.".to_string(),
                ));
            }
            // 6. Transfer admin fee to admin
            let admin_fee: Balance = Balance::from(competition.competitors_count)
                * (U256::from(competition.entry_fee_amount)
                    * U256::from(competition.admin_fee_percentage_numerator)
                    / U256::from(PERCENTAGE_CALCULATION_DENOMINATOR))
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

        #[ink(message)]
        pub fn collect_prize(&mut self, id: u64, token: AccountId) -> Result<Balance> {
            // 1. Get competition
            let competition: Competition = self.competitions_show(id)?;
            // 2. Validate that all competitors have been placed
            if competition.competitors_count != competition.competitors_placed_count {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "All competitors haven't been placed yet.".to_string(),
                ));
            }
            // 3. Get CompetitionTokenCompetitor
            let caller: AccountId = Self::env().caller();
            let mut competition_token_competitor: CompetitionTokenCompetitor =
                self.competition_token_competitors_show(id, token, caller)?;
            // 4. Validate prize hasn't been collected yet
            if competition_token_competitor.collected {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Prize has already been collected.".to_string(),
                ));
            }
            // 5. Get competition token prize
            let mut competition_token_prize: CompetitionTokenPrize =
                self.competition_token_prizes_show(id, token)?;
            // 6. Get competitor
            let competitor: Competitor = self.competitors_show(competition.id, caller)?;
            // 7. Get PlaceDetail for user
            let competition_place_details_vec: Vec<CompetitionPlaceDetail> =
                self.competition_place_details.get(id).unwrap();
            let competition_place_details_index_as_usize: usize =
                usize::try_from(competitor.competition_place_details_index).unwrap();
            let competition_place_detail: &CompetitionPlaceDetail =
                &competition_place_details_vec[competition_place_details_index_as_usize];
            // 8. Calculate prize available
            let prize_available: Balance =
                competition_token_prize.amount - competition_token_prize.collected;
            // 9. Calculate amount of token to send to user
            let mut amount_to_send_to_user: Balance =
                (U256::from(competition_place_detail.payout_numerator)
                    * U256::from(prize_available)
                    / U256::from(PERCENTAGE_CALCULATION_DENOMINATOR)
                    / U256::from(competition_place_detail.competitors_count))
                .as_u128();
            if amount_to_send_to_user > prize_available {
                amount_to_send_to_user = prize_available
            }
            // 10. validate that amount_to_send_to_user is greater than zero
            if amount_to_send_to_user == 0 {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "No prize to collect.".to_string(),
                ));
            }

            // 11. Send token to user
            PSP22Ref::transfer_builder(&token, caller, amount_to_send_to_user, vec![])
                .call_flags(CallFlags::default())
                .invoke()?;
            // 12. Set collected to true
            competition_token_competitor.collected = true;
            self.competition_token_competitors
                .insert((id, token, caller), &competition_token_competitor);
            // 13. Update CompetitionTokenPrize
            competition_token_prize.collected += amount_to_send_to_user;
            self.competition_token_prizes
                .insert((id, token), &competition_token_prize);

            // emit event
            Self::emit_event(
                self.env(),
                Event::CollectPrize(CollectPrize {
                    id,
                    competitor: caller,
                    token,
                    amount: amount_to_send_to_user,
                }),
            );

            Ok(amount_to_send_to_user)
        }

        #[ink(message)]
        pub fn competitions_create(
            &mut self,
            start: Timestamp,
            end: Timestamp,
            entry_fee_token: AccountId,
            entry_fee_amount: Balance,
            admin_fee_percentage_numerator: Option<u16>,
            azero_processing_fee: Option<Balance>,
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
                azero_processing_fee: azero_processing_fee
                    .unwrap_or(self.default_azero_processing_fee),
                judge: self.admin,
                // has to start at 1 as all competitors start at 0
                judge_place_attempt: 1,
                next_judge: None,
                payout_places: 0,
                payout_structure_numerator_sum: 0,
                creator: caller,
                token_prices_vec: vec![],
                competitors_count: 0,
                competitor_final_value_updated_count: 0,
                competitors_placed_count: 0,
            };
            self.competitions
                .insert(self.competitions_count, &competition);
            self.competitions_count += 1;
            self.competition_judges.insert(
                (competition.id, competition.judge),
                &CompetitionJudge {
                    deadline: competition.end + DAY_IN_MS,
                },
            );

            self.competition_place_details
                .insert::<u64, std::vec::Vec<CompetitionPlaceDetail>>(competition.id, &vec![]);

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
                    azero_processing_fee: competition.azero_processing_fee,
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
            if competition.competitors_count > 0 {
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
                } else {
                    competition.payout_places = 99
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

        // This isn't the final USD value as it doesn't factor in each token's decimal points.
        // Doesn't matter though as it can still be used to find out who the winners are.
        #[ink(message)]
        pub fn competitor_final_value_update(
            &mut self,
            id: u64,
            competitor_address: AccountId,
        ) -> Result<String> {
            // 1. Get competition
            let mut competition: Competition = self.competitions_show(id)?;
            // 2. Validate competition has ended
            self.validate_competition_has_ended(competition.clone())?;
            // 3. Get Competitor
            let mut competitor: Competitor = self.competitors_show(id, competitor_address)?;
            // 4. Validate Competitor hasn't been processed
            if competitor.final_value.is_some() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competitor already processed.".to_string(),
                ));
            }
            // 5. Validate competition token prices have been set
            if competition.token_prices_vec.is_empty() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Token prices haven't been set.".to_string(),
                ));
            }

            // 6. Calculate usd value and add token balance to competition prizes
            let mut competitor_value: U256 = U256::from(0);
            for dia_price_symbol in VALID_DIA_PRICE_SYMBOLS.iter() {
                let token: AccountId = self
                    .dia_price_symbol_tokens_mapping
                    .get(dia_price_symbol.to_string())
                    .unwrap();
                let price: Balance = self
                    .competition_token_prices
                    .get((competition.id, token))
                    .unwrap();
                let competition_token_competitor: CompetitionTokenCompetitor = self
                    .competition_token_competitors
                    .get((id, token, competitor_address))
                    .unwrap();
                if competition_token_competitor.amount > 0 {
                    competitor_value +=
                        U256::from(price) * U256::from(competition_token_competitor.amount);
                    let mut competition_token_prize: CompetitionTokenPrize = self
                        .competition_token_prizes
                        .get((competition.id, token))
                        .unwrap_or(CompetitionTokenPrize {
                            amount: 0,
                            collected: 0,
                        });
                    competition_token_prize.amount += competition_token_competitor.amount;
                    self.competition_token_prizes
                        .insert((competition.id, token), &competition_token_prize);
                }
            }
            // 7. Set final_value
            let competitor_value_as_string: String = competitor_value.to_string();
            competitor.final_value = Some(competitor_value_as_string.clone());
            self.competitors
                .insert((id, competitor_address), &competitor);
            // 8. Increase competition.competitor_final_value_updated_count
            competition.competitor_final_value_updated_count += 1;
            self.competitions.insert(competition.id, &competition);
            // 9. Send processing fee to caller
            let processing_fee: Balance = (U256::from(competition.azero_processing_fee)
                * U256::from(FINAL_VALUE_UPDATE_FEE_PERCENTAGE_NUMERATOR)
                / U256::from(PERCENTAGE_CALCULATION_DENOMINATOR))
            .as_u128();
            if processing_fee > 0
                && self
                    .env()
                    .transfer(Self::env().caller(), processing_fee)
                    .is_err()
            {
                panic!(
                    "requested transfer failed. this can be the case if the contract does not\
                         have sufficient free funds or if the transfer would have brought the\
                         contract's balance below minimum balance."
                )
            }

            // emit event
            Self::emit_event(
                self.env(),
                Event::CompetitorFinalValueUpdate(CompetitorFinalValueUpdate {
                    id: competition.id,
                    competitor: competitor_address,
                    value: competitor_value_as_string.clone(),
                }),
            );

            Ok(competitor_value_as_string)
        }

        #[ink(message)]
        pub fn deregister(&mut self, id: u64) -> Result<()> {
            // 1. Get competition
            let mut competition: Competition = self.competitions_show(id)?;
            // 2. Validate that caller is registered
            let caller: AccountId = Self::env().caller();
            self.competition_token_competitors_show(id, competition.entry_fee_token, caller)?;
            // 3. Validate able to deregister
            if Self::env().block_timestamp() >= competition.start
                && competition.competitors_count >= competition.payout_places.into()
            {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Unable to deregister when competition has started and minimum competitor requirements met.".to_string(),
                ));
            }

            // 4. Transfer token back to caller
            PSP22Ref::transfer_builder(
                &competition.entry_fee_token,
                caller,
                competition.entry_fee_amount,
                vec![],
            )
            .call_flags(CallFlags::default())
            .invoke()?;
            // 5. Remove competition token competitors
            for (_index, token_to_dia_price_symbol_combo) in
                self.token_dia_price_symbols_vec.iter().enumerate()
            {
                self.competition_token_competitors.remove((
                    id,
                    token_to_dia_price_symbol_combo.0,
                    caller,
                ));
            }
            // 6. Update competition
            competition.competitors_count -= 1;
            self.competitions.insert(id, &competition);
            // 7. Transfer funds to buyer
            if self
                .env()
                .transfer(caller, competition.azero_processing_fee)
                .is_err()
            {
                panic!(
                    "requested transfer failed. this can be the case if the contract does not\
                     have sufficient free funds or if the transfer would have brought the\
                     contract's balance below minimum balance."
                )
            }

            // emit event
            Self::emit_event(
                self.env(),
                Event::Deregister(Deregister {
                    id,
                    competitor: caller,
                }),
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
        pub fn place_competitors(
            &mut self,
            id: u64,
            competitors_addresses: Vec<AccountId>,
        ) -> Result<()> {
            // 1. Get competition
            let mut competition: Competition = self.competitions_show(id)?;
            // 2. Validate that the caller is the judge
            if competition.judge != Self::env().caller() {
                return Err(AzTradingCompetitionError::Unauthorised);
            }
            // 3. Validate that the competition has ended
            self.validate_competition_has_ended(competition.clone())?;
            // 4. Validate that all competitors have had their final values set
            if competition.competitors_count != competition.competitor_final_value_updated_count {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "All competitors have not had their final values updated.".to_string(),
                ));
            }
            let mut competition_place_details_vec: Vec<CompetitionPlaceDetail> =
                self.competition_place_details.get(competition.id).unwrap();
            // 5. Go through competitors
            for competitor_address in competitors_addresses.iter() {
                // 6a. Validate that competitor_address belongs to a competitor
                // 6b. Validate that competitor hasn't been placed yet
                if let Some(mut competitor_unwrapped) =
                    self.competitors.get((id, competitor_address))
                {
                    if competitor_unwrapped.judge_place_attempt == competition.judge_place_attempt {
                        return Err(AzTradingCompetitionError::UnprocessableEntity(
                            "Competitor has already been placed.".to_string(),
                        ));
                    }

                    let competitor_final_value: String =
                        competitor_unwrapped.final_value.clone().unwrap();
                    // 7. Place competitor by checking place_details_ordered_by_competitor_final_value
                    let competition_place_details_vec_len = competition_place_details_vec.len();
                    let payout_numerator: u16 =
                        self.payout_numerator_for_next_place(competition.clone());
                    let mut place_index: u32 =
                        competition_place_details_vec_len.try_into().unwrap();
                    if competition_place_details_vec_len == 0 {
                        competition_place_details_vec.push(CompetitionPlaceDetail {
                            competitor_value: competitor_final_value,
                            competitors_count: 1,
                            payout_numerator,
                        });
                    } else {
                        let latest_placed_price = U256::from_dec_str(
                            &competition_place_details_vec[competition_place_details_vec_len - 1]
                                .competitor_value,
                        )
                        .unwrap();
                        let competitor_final_value =
                            U256::from_dec_str(&competitor_final_value).unwrap();
                        if latest_placed_price == competitor_final_value {
                            // Add to the count
                            competition_place_details_vec[competition_place_details_vec_len - 1]
                                .competitors_count += 1;
                            // Add to the payout_numerator
                            competition_place_details_vec[competition_place_details_vec_len - 1]
                                .payout_numerator += payout_numerator;
                            place_index = place_index - 1;
                        } else if competitor_final_value > latest_placed_price {
                            competition_place_details_vec.push(CompetitionPlaceDetail {
                                competitor_value: competitor_final_value.to_string(),
                                competitors_count: 1,
                                payout_numerator,
                            });
                        } else {
                            return Err(AzTradingCompetitionError::UnprocessableEntity(
                                "Competitor is in the wrong place.".to_string(),
                            ));
                        }
                    }
                    // 8. Update judge place attempt and place_detail_index
                    competitor_unwrapped.judge_place_attempt = competition.judge_place_attempt;
                    competitor_unwrapped.competition_place_details_index = place_index;
                    self.competitors
                        .insert((id, competitor_address), &competitor_unwrapped);
                    // 9. Increase competitor placed count
                    competition.competitors_placed_count += 1;
                } else {
                    return Err(AzTradingCompetitionError::NotFound(
                        "Competitor".to_string(),
                    ));
                }
            }

            // 10. Update competition
            self.competitions.insert(competition.id, &competition);

            // 11. Update competition_place_details
            self.competition_place_details
                .insert(competition.id, &competition_place_details_vec);

            // emit event
            Self::emit_event(
                self.env(),
                Event::PlaceCompetitor(PlaceCompetitor {
                    id: competition.id,
                    competitors_addresses,
                }),
            );

            Ok(())
        }

        // This can be called by anyone
        #[ink(message)]
        pub fn judge_update(&mut self, id: u64) -> Result<()> {
            // 1. Get competition
            let mut competition: Competition = self.competitions_show(id)?;
            // 2. Validate that competitor's haven't been placed yet
            self.validate_all_competitors_have_not_been_placed(competition.clone())?;
            // 3. Validate that next judge exists
            if let Some(next_judge_unwrapped) = competition.next_judge {
                let current_timestamp: Timestamp = Self::env().block_timestamp();
                let current_judge_deadline: Timestamp = self
                    .competition_judges
                    .get((id, competition.judge))
                    .unwrap()
                    .deadline;
                // 4. Validate that the current timestamp is after current judge deadline and before next judge deadline
                if current_timestamp <= current_judge_deadline {
                    return Err(AzTradingCompetitionError::UnprocessableEntity(
                        "Current judge deadline hasn't passed.".to_string(),
                    ));
                }

                // 5. Update judge and next_judge
                competition.judge = next_judge_unwrapped;
                competition.next_judge = None;
                self.competitions.insert(id, &competition);
            } else {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Next judge absent.".to_string(),
                ));
            };

            // emit event
            Self::emit_event(
                self.env(),
                Event::JudgeUpdate(JudgeUpdate {
                    id: competition.id,
                    judge: competition.judge,
                }),
            );

            Ok(())
        }

        #[ink(message)]
        pub fn next_judge_update(&mut self, id: u64) -> Result<Competition> {
            let caller: AccountId = Self::env().caller();
            // 1. Get competition
            let mut competition: Competition = self.competitions_show(id)?;
            // 2. Validate that all competitors haven't been placed yet
            self.validate_all_competitors_have_not_been_placed(competition.clone())?;
            // 3. Validate that caller hasn't been a competition judge yet
            if self.competition_judges.get((id, caller)).is_some() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "You can only be a judge one time.".to_string(),
                ));
            }
            // 4. Validate that caller performed better next judge in specified competition
            if let Some(next_judge_unwrapped) = competition.next_judge {
                let mut caller_final_value = U256::from(0);
                let mut next_judge_final_value = U256::from(0);
                if let Some(caller_competitor_unwrapped) = self.competitors.get((id, caller)) {
                    caller_final_value = U256::from_dec_str(
                        &caller_competitor_unwrapped
                            .final_value
                            .unwrap_or("0".to_string()),
                    )
                    .unwrap()
                }
                if let Some(next_judge_competitor_unwrapped) =
                    self.competitors.get((id, next_judge_unwrapped))
                {
                    next_judge_final_value = U256::from_dec_str(
                        &next_judge_competitor_unwrapped
                            .final_value
                            .unwrap_or("0".to_string()),
                    )
                    .unwrap()
                }
                if caller_final_value <= next_judge_final_value {
                    return Err(AzTradingCompetitionError::UnprocessableEntity(
                        "Next judge can only be replaced by callers that performed better in specified competition.".to_string(),
                    ));
                }

                // Remove former next judge from competition judges
                self.competition_judges.remove((id, next_judge_unwrapped));
            };

            // 5. Set next judge
            competition.next_judge = Some(caller);
            self.competitions.insert(id, &competition);
            // 6. Set competition judge
            let current_judge_deadline: Timestamp = self
                .competition_judges
                .get((competition.id, competition.judge))
                .unwrap()
                .deadline;
            let deadline: Timestamp = if Self::env().block_timestamp() > current_judge_deadline {
                Self::env().block_timestamp() + DAY_IN_MS
            } else {
                current_judge_deadline + DAY_IN_MS
            };
            self.competition_judges
                .insert((id, caller), &CompetitionJudge { deadline });

            // emit event
            Self::emit_event(
                self.env(),
                Event::NextJudgeUpdate(NextJudgeUpdate {
                    id: competition.id,
                    user: caller,
                }),
            );

            Ok(competition)
        }

        #[ink(message, payable)]
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
            // 3. Validate that caller hasn't registered already
            let caller: AccountId = Self::env().caller();
            if self
                .competition_token_competitors
                .get((id, competition.entry_fee_token, caller))
                .is_some()
            {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Already registered".to_string(),
                ));
            }
            // 4. Validate that azero processing fee has been paid
            if self.env().transferred_value() != competition.azero_processing_fee {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Please include AZERO processing fee.".to_string(),
                ));
            }

            // 5. Acquire token from caller
            self.acquire_psp22(
                competition.entry_fee_token,
                caller,
                competition.entry_fee_amount,
            )?;
            // 6. Figure out admin fee
            let admin_fee: Balance = self.admin_fee(&competition);
            // 7. Create all CompetitionTokenCompetitors for competitor
            for (_index, token_to_dia_price_symbol_combo) in
                self.token_dia_price_symbols_vec.iter().enumerate()
            {
                let token_balance: Balance =
                    if competition.entry_fee_token == token_to_dia_price_symbol_combo.0 {
                        competition.entry_fee_amount - admin_fee
                    } else {
                        0
                    };
                self.competition_token_competitors.insert(
                    (competition.id, token_to_dia_price_symbol_combo.0, caller),
                    &CompetitionTokenCompetitor {
                        amount: token_balance,
                        collected: false,
                    },
                );
            }
            // 8. Increase competition.competitors_count
            competition.competitors_count += 1;
            self.competitions.insert(competition.id, &competition);
            // 9. Create Competitor
            self.competitors.insert(
                (competition.id, caller),
                &Competitor {
                    final_value: None,
                    judge_place_attempt: 0,
                    competition_place_details_index: 0,
                },
            );

            // emit event
            Self::emit_event(
                self.env(),
                Event::Register(Register {
                    id,
                    competitor: caller,
                }),
            );

            Ok(())
        }

        // This needs to be called when:
        // 1. The judge wants to reset
        #[ink(message)]
        pub fn reset(&mut self, id: u64) -> Result<()> {
            let mut competition: Competition = self.competitions_show(id)?;
            let caller: AccountId = Self::env().caller();
            Self::authorise(competition.judge, caller)?;
            if competition.judge_place_attempt == u128::MAX {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Maximum place attempts reached.".to_string(),
                ));
            }
            if competition.competitors_placed_count == 0 {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Zero competitors have been placed.".to_string(),
                ));
            }
            self.validate_all_competitors_have_not_been_placed(competition.clone())?;

            competition.competitors_placed_count = 0;
            competition.judge_place_attempt += 1;
            self.competitions.insert(competition.id, &competition);
            self.competition_place_details
                .insert::<u64, std::vec::Vec<CompetitionPlaceDetail>>(competition.id, &vec![]);

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
            // 1. Validate that there's enough competitors in competition
            if competition.competitors_count < competition.payout_places.into() {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition is invalid, please deregister.".to_string(),
                ));
            }
            // 2. Validate that competition is in progress
            self.validate_competition_is_in_progress(competition.clone())?;
            // 3. Validate that competitor has enough to cover amount_in
            let caller: AccountId = Self::env().caller();
            let mut in_competition_token_competitor: CompetitionTokenCompetitor =
                self.competition_token_competitors_show(id, in_token, caller)?;
            if amount_in > in_competition_token_competitor.amount {
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
            // 7. Adjust competitor balances
            // Decrease amount_in for competition token competitor
            in_competition_token_competitor.amount -= amount_in;
            self.competition_token_competitors
                .insert((id, in_token, caller), &in_competition_token_competitor);
            // Increase received amount for competition token caller
            let mut out_competition_token_competitor: CompetitionTokenCompetitor =
                self.competition_token_competitors_show(id, out_token, caller)?;
            out_competition_token_competitor.amount += out_amount;
            self.competition_token_competitors
                .insert((id, out_token, caller), &out_competition_token_competitor);

            // emit event
            Self::emit_event(
                self.env(),
                Event::Swap(Swap {
                    id,
                    competitor: caller,
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

        fn admin_fee(&self, competition: &Competition) -> Balance {
            (U256::from(competition.entry_fee_amount)
                * U256::from(competition.admin_fee_percentage_numerator)
                / U256::from(DEFAULT_ADMIN_FEE_PERCENTAGE_NUMERATOR))
            .as_u128()
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

        fn payout_numerator_for_next_place(&self, competition: Competition) -> u16 {
            if competition.competitors_placed_count < competition.payout_places.into() {
                let competitors_placed_count_as_u16: u16 =
                    competition.competitors_placed_count.try_into().unwrap();
                self.competition_payout_structure_numerators
                    .get((competition.id, competitors_placed_count_as_u16))
                    .unwrap()
            } else {
                0
            }
        }

        fn validate_all_competitors_have_not_been_placed(
            &self,
            competition: Competition,
        ) -> Result<()> {
            if competition.competitors_placed_count == competition.competitors_count {
                return Err(AzTradingCompetitionError::UnprocessableEntity(
                    "All competitors have been placed.".to_string(),
                ));
            }

            Ok(())
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
        const MOCK_DEFAULT_AZERO_PROCESSING_FEE: Balance = 1_000_000_000_000;
        const MOCK_ENTRY_FEE_AMOUNT: Balance = 555_555;
        const MOCK_START: Timestamp = 654_654;
        const MOCK_END: Timestamp = 754_654;

        // === HELPERS ===
        fn init() -> (DefaultAccounts<DefaultEnvironment>, AzTradingCompetition) {
            let accounts = default_accounts();
            set_caller::<DefaultEnvironment>(accounts.bob);
            let az_trading_competition = AzTradingCompetition::new(
                mock_allowed_pair_token_combinations(),
                MOCK_DEFAULT_AZERO_PROCESSING_FEE,
                mock_dia_address(),
                mock_router_address(),
                mock_token_to_dia_price_symbol_combos(),
            );
            (accounts, az_trading_competition.expect("REASON"))
        }

        fn contract_id() -> AccountId {
            ink::env::test::callee::<ink::env::DefaultEnvironment>()
        }

        fn get_balance(account_id: AccountId) -> Balance {
            ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(account_id)
                .expect("Cannot get account balance")
        }

        fn set_balance(account_id: AccountId, balance: Balance) {
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(account_id, balance)
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
            assert_eq!(
                config.default_azero_processing_fee,
                MOCK_DEFAULT_AZERO_PROCESSING_FEE
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

        #[ink::test]
        fn test_competition_place_details_show() {
            let (_accounts, mut az_trading_competition) = init();
            // when CompetitionPlaceDetail does not exist
            let result = az_trading_competition.competition_place_details_show(0, 0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "CompetitionPlaceDetail".to_string(),
                ))
            );
            // when CompetitionPlaceDetail exists
            let competition_place_detail: CompetitionPlaceDetail = CompetitionPlaceDetail {
                competitor_value: "0".to_string(),
                competitors_count: 1,
                payout_numerator: 1,
            };
            az_trading_competition
                .competition_place_details
                .insert(0, &vec![competition_place_detail.clone()]);
            // = when called with the correct index
            // = * it returns the CompetitionPlaceDetail
            let result: CompetitionPlaceDetail = az_trading_competition
                .competition_place_details_show(0, 0)
                .unwrap();
            assert_eq!(result, competition_place_detail);
            // = when called with the incorrect index
            // = * it raises an error
            let result = az_trading_competition.competition_place_details_show(0, 1);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "CompetitionPlaceDetail".to_string(),
                ))
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
            // === when competition has not met minimum competitor requirements
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
                    "Competition hasn't met minimum competitor requirements.".to_string(),
                ))
            );
            // === when competition has met minimum competitor requirements
            competition.competitors_count = (competition.payout_places + 1).into();
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
        fn test_collect_prize() {
            let (accounts, mut az_trading_competition) = init();
            // = when competition does not exist
            // = * it raises an error
            let result = az_trading_competition
                .collect_prize(0, mock_token_to_dia_price_symbol_combos()[0].0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
            );
            // = when competition exists
            let mut competition: Competition = az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                    None,
                    None,
                )
                .unwrap();
            // == when all competitors haven't been placed yet
            competition.competitors_count = 1;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // == * it raises an error
            let result = az_trading_competition
                .collect_prize(competition.id, mock_token_to_dia_price_symbol_combos()[0].0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "All competitors haven't been placed yet.".to_string(),
                ))
            );
            // == when all competitors have been placed
            competition.competitors_placed_count = competition.competitors_count;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // === when competition token competitor is not present
            // === * it raises an error
            let result = az_trading_competition
                .collect_prize(competition.id, mock_token_to_dia_price_symbol_combos()[0].0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "CompetitionTokenCompetitor".to_string(),
                ))
            );
            // === when competition token competitor is present
            // ==== when prize has been collected already
            let mut competition_token_competitor: CompetitionTokenCompetitor =
                CompetitionTokenCompetitor {
                    amount: 0,
                    collected: true,
                };
            az_trading_competition.competition_token_competitors.insert(
                (
                    competition.id,
                    mock_token_to_dia_price_symbol_combos()[0].0,
                    accounts.bob,
                ),
                &competition_token_competitor,
            );
            // ==== * it raises an error
            let result = az_trading_competition
                .collect_prize(competition.id, mock_token_to_dia_price_symbol_combos()[0].0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Prize has already been collected.".to_string(),
                ))
            );
            // ==== when prize has not been collected yet
            competition_token_competitor.collected = false;
            az_trading_competition.competition_token_competitors.insert(
                (
                    competition.id,
                    mock_token_to_dia_price_symbol_combos()[0].0,
                    accounts.bob,
                ),
                &competition_token_competitor,
            );
            // ===== when competition token prize doesn't exist
            // ===== * it raises an error
            let result = az_trading_competition
                .collect_prize(competition.id, mock_token_to_dia_price_symbol_combos()[0].0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "CompetitionTokenPrize".to_string(),
                ))
            );
            // ==== when competition token prize exists
            let mut competition_token_prize: CompetitionTokenPrize = CompetitionTokenPrize {
                amount: 5,
                collected: 0,
            };
            az_trading_competition.competition_token_prizes.insert(
                (competition.id, mock_token_to_dia_price_symbol_combos()[0].0),
                &competition_token_prize,
            );
            // ===== when competitor's place detail numerator is zero
            az_trading_competition.competitors.insert(
                (competition.id, accounts.bob),
                &Competitor {
                    final_value: Some("1".to_string()),
                    judge_place_attempt: 1,
                    competition_place_details_index: 0,
                },
            );
            let mut competition_place_details_vec = az_trading_competition
                .competition_place_details
                .get(competition.id)
                .unwrap();
            let mut competition_place_detail: CompetitionPlaceDetail = CompetitionPlaceDetail {
                competitor_value: "1".to_string(),
                competitors_count: 1,
                payout_numerator: 0,
            };
            competition_place_details_vec.push(competition_place_detail.clone());
            az_trading_competition
                .competition_place_details
                .insert(competition.id, &competition_place_details_vec);
            // ===== * it raises an error
            let result = az_trading_competition
                .collect_prize(competition.id, mock_token_to_dia_price_symbol_combos()[0].0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "No prize to collect.".to_string(),
                ))
            );
            // ===== when place detail numerator is positive
            competition_place_detail.payout_numerator = 1;
            competition_place_details_vec.pop();
            competition_place_details_vec.push(competition_place_detail);
            az_trading_competition
                .competition_place_details
                .insert(competition.id, &competition_place_details_vec);
            // ====== when competition token prize has been fully colleted already
            competition_token_prize.collected = competition_token_prize.amount;
            az_trading_competition.competition_token_prizes.insert(
                (competition.id, mock_token_to_dia_price_symbol_combos()[0].0),
                &competition_token_prize,
            );
            let result = az_trading_competition
                .collect_prize(competition.id, mock_token_to_dia_price_symbol_combos()[0].0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "No prize to collect.".to_string(),
                ))
            );
            // ====== when competition token prize has not been fully collected already
            competition_token_prize.collected = 1;
            az_trading_competition.competition_token_prizes.insert(
                (competition.id, mock_token_to_dia_price_symbol_combos()[0].0),
                &competition_token_prize,
            );
            // ======= when amount to send to user is zero
            // ======= * it raises an error
            let result = az_trading_competition
                .collect_prize(competition.id, mock_token_to_dia_price_symbol_combos()[0].0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "No prize to collect.".to_string(),
                ))
            );
            // ======= when amount to send to user is positive
            // ======= will have to do in integration tests because of sending tokens
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
                None,
            );
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Max number of competitions reached.".to_string(),
                ))
            );
            // when competitions_count is less than u64 max
            az_trading_competition.competitions_count = u64::MAX - 3;
            // = when duration is less than or equal to MINIMUM_DURATION
            let result = az_trading_competition.competitions_create(
                MOCK_START,
                MOCK_START + MINIMUM_DURATION - 1,
                mock_entry_fee_token(),
                MOCK_ENTRY_FEE_AMOUNT,
                None,
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
            let mut competitions_count: u64 = az_trading_competition.competitions_count;
            // === when fee token doesn't have a dia price symbol
            let result = az_trading_competition.competitions_create(
                MOCK_START,
                MOCK_START + MINIMUM_DURATION,
                mock_dia_address(),
                MOCK_ENTRY_FEE_AMOUNT,
                None,
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
                    None,
                )
                .unwrap();
            // ==== when azero_processing_fee is not present
            // ==== * it stores the competition with the default_azero_processing_fee
            let mut competition = az_trading_competition
                .competitions
                .get(&competitions_count)
                .unwrap();
            assert_eq!(
                competition.azero_processing_fee,
                MOCK_DEFAULT_AZERO_PROCESSING_FEE
            );
            // ==== when azero_processing_fee is present
            // ==== * it stores the competition with the provided azero_processing_fee
            competition = az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                    None,
                    Some(MOCK_DEFAULT_AZERO_PROCESSING_FEE - 1),
                )
                .unwrap();
            competitions_count += 1;
            assert_eq!(
                competition.azero_processing_fee,
                MOCK_DEFAULT_AZERO_PROCESSING_FEE - 1
            );
            // ==== when admin_fee_percentage_numerator is not present
            // ==== * it stores the competition with default fee percentage numerator
            assert_eq!(
                competition.admin_fee_percentage_numerator,
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
                None,
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
                None,
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
                    None,
                )
                .unwrap();
            let competition: Competition = az_trading_competition
                .competitions
                .get(&competitions_count + 1)
                .unwrap();
            assert_eq!(
                competition.admin_fee_percentage_numerator,
                admin_fee_percentage_numerator.unwrap(),
            );
            // ======= * it sets the admin as the judge
            assert_eq!(competition.judge, az_trading_competition.admin,);
            // ======= * it stores the competition judge for the admin with the deadline 1 day after the competition end
            assert_eq!(
                competition.end + DAY_IN_MS,
                az_trading_competition
                    .competition_judges
                    .get((competition.id, competition.judge))
                    .unwrap()
                    .deadline
            )
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
            competition.competitors_count = 1;
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
            competition.competitors_count = 0;
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
                    None,
                )
                .unwrap();
            // = when caller is not registered
            // = * it raises an error
            let result = az_trading_competition.deregister(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "CompetitionTokenCompetitor".to_string(),
                ))
            );
            // = when caller is registered
            az_trading_competition.competition_token_competitors.insert(
                (0, mock_entry_fee_token(), accounts.bob),
                &CompetitionTokenCompetitor {
                    amount: MOCK_ENTRY_FEE_AMOUNT,
                    collected: false,
                },
            );
            // == when competition has started
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START);
            // === when competitor count is equal to or greater than payout places
            // === * it raises an error
            let result = az_trading_competition.deregister(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Unable to deregister when competition has started and minimum competitor requirements met.".to_string(),
                ))
            );
            // == NEEDS TO BE DONE IN INTEGRATION TESTS
            // === when competitor count is less than the amount of payout places
            // == when competition hasn't started
            // == * it sends the entry fee back to caller
            // == * it removes competition token competitor
            // == * it decreases the competitor count
        }

        #[ink::test]
        fn test_competitor_final_value_update() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.competitor_final_value_update(0, accounts.bob);
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
                    None,
                )
                .unwrap();
            // = when competition hasn't ended
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(competition.end);
            // = * it raises an error
            let result = az_trading_competition.competitor_final_value_update(0, accounts.bob);
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
            // == when Competitor doesn't exist
            let result = az_trading_competition.competitor_final_value_update(0, accounts.bob);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competitor".to_string(),
                ))
            );
            // == when Competitor exists
            let mut competitor: Competitor = Competitor {
                final_value: Some(0.to_string()),
                judge_place_attempt: 0,
                competition_place_details_index: 0,
            };
            az_trading_competition
                .competitors
                .insert((0, accounts.bob), &competitor);
            // === when Competitor is processed already
            // === * it raises an error
            let result = az_trading_competition.competitor_final_value_update(0, accounts.bob);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competitor already processed.".to_string(),
                ))
            );
            // === when Competitor doesn't have final_value
            competitor.final_value = None;
            az_trading_competition
                .competitors
                .insert((0, accounts.bob), &competitor);
            // ==== when competion token prices haven't been set
            // ==== * it raises an error
            let result = az_trading_competition.competitor_final_value_update(0, accounts.bob);
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
            let mut competitor_usd_value: Balance = 0;
            let token_balance: Balance = 1;
            for (index, mock_token_to_dia_price_symbol_combo) in
                mock_token_to_dia_price_symbol_combos().iter().enumerate()
            {
                az_trading_competition.competition_token_prices.insert(
                    (competition.id, mock_token_to_dia_price_symbol_combo.0),
                    &competition.token_prices_vec[index].1,
                );
                az_trading_competition.competition_token_competitors.insert(
                    (
                        competition.id,
                        mock_token_to_dia_price_symbol_combo.0,
                        accounts.bob,
                    ),
                    &CompetitionTokenCompetitor {
                        amount: token_balance,
                        collected: false,
                    },
                );
                competitor_usd_value += competition.token_prices_vec[index].1
            }
            set_balance(
                contract_id(),
                MOCK_DEFAULT_AZERO_PROCESSING_FEE * 100 / 1000,
            );
            let caller_balance: Balance = get_balance(accounts.bob);
            az_trading_competition
                .competitor_final_value_update(0, accounts.bob)
                .unwrap();
            // ==== * it sets the final_value for the competitor
            let final_value: String = az_trading_competition
                .competitors
                .get((competition.id, accounts.bob))
                .unwrap()
                .final_value
                .unwrap();
            assert_eq!(final_value, competitor_usd_value.to_string());
            // ==== * it adds to the competition_token_prize
            for (_index, mock_token_to_dia_price_symbol_combo) in
                mock_token_to_dia_price_symbol_combos().iter().enumerate()
            {
                assert_eq!(
                    az_trading_competition
                        .competition_token_prizes_show(
                            competition.id,
                            mock_token_to_dia_price_symbol_combo.0
                        )
                        .unwrap()
                        .amount,
                    token_balance
                );
            }
            // ==== * it increases the competition.competitor_final_value_updated_count by one
            competition = az_trading_competition
                .competitions
                .get(competition.id)
                .unwrap();
            assert_eq!(competition.competitor_final_value_updated_count, 1);
            // ==== * it sends the caller 10% of the azero_processing_fee
            assert!(get_balance(accounts.bob) > caller_balance);
            assert!(
                get_balance(accounts.bob)
                    < (caller_balance + MOCK_DEFAULT_AZERO_PROCESSING_FEE * 110 / 1000)
            );
            assert_eq!(0, get_balance(contract_id()))
        }

        #[ink::test]
        fn test_judge_update() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.judge_update(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
            );
            // when competition exist
            let mut competition: Competition = az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                    None,
                    None,
                )
                .unwrap();
            // = when all of the competitors have been placed
            competition.competitors_count = 5;
            competition.competitors_placed_count = 5;
            az_trading_competition.competitions.insert(0, &competition);
            // = * it raises an error
            let result = az_trading_competition.judge_update(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "All competitors have been placed.".to_string(),
                ))
            );
            // = when all of the competitors haven't been placed
            competition.competitors_count = 5;
            competition.competitors_placed_count = 1;
            az_trading_competition.competitions.insert(0, &competition);
            // == when next judge does not exist
            // == * it raises an error
            let result = az_trading_competition.judge_update(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Next judge absent.".to_string(),
                ))
            );
            // == when next judge exists
            competition.next_judge = Some(accounts.django);
            az_trading_competition.competitions.insert(0, &competition);
            // === when current time is before or equal to current judge deadline
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START);
            az_trading_competition.competition_judges.insert(
                (competition.id, competition.judge),
                &CompetitionJudge {
                    deadline: MOCK_START,
                },
            );
            // === * it raises an error
            let result = az_trading_competition.judge_update(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Current judge deadline hasn't passed.".to_string(),
                ))
            );
            // === when current time is after current judge deadline
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(MOCK_START + 1);
            // === * it updates the judge
            az_trading_competition.judge_update(0).unwrap();
            competition = az_trading_competition.competitions.get(0).unwrap();
            assert_eq!(competition.judge, accounts.django);
            // === * it resets the next_judge
            assert_eq!(competition.next_judge, None);
        }

        #[ink::test]
        fn test_next_judge_update() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.next_judge_update(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
            );
            // when competition exist
            let mut competition: Competition = az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                    None,
                    None,
                )
                .unwrap();
            // = when all of the competitors have been placed
            competition.competitors_count = 5;
            competition.competitors_placed_count = 5;
            az_trading_competition.competitions.insert(0, &competition);
            // = * it raises an error
            let result = az_trading_competition.next_judge_update(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "All competitors have been placed.".to_string(),
                ))
            );
            // = when all of the competitors haven't been placed
            competition.competitors_count = 5;
            competition.competitors_placed_count = 1;
            az_trading_competition.competitions.insert(0, &competition);
            // == when caller has been a competition judge before
            // == * it raises an error
            let result = az_trading_competition.next_judge_update(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "You can only be a judge one time.".to_string(),
                ))
            );
            // == when caller has not been competition judge before
            set_caller::<DefaultEnvironment>(accounts.charlie);
            // === when next_judge is present
            competition.next_judge = Some(accounts.django);
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // ==== when caller's final value in competition is less than or equal to next judge
            // ==== * it raises an error
            let result = az_trading_competition.next_judge_update(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Next judge can only be replaced by callers that performed better in specified competition.".to_string(),
                ))
            );
            // ==== when caller's final value in competition is more than next judge
            az_trading_competition.competitors.insert(
                (competition.id, accounts.charlie),
                &Competitor {
                    final_value: Some("1".to_string()),
                    judge_place_attempt: 0,
                    competition_place_details_index: 0,
                },
            );
            // ==== * it replaces the current next_judge with the caller
            competition = az_trading_competition.next_judge_update(0).unwrap();
            assert_eq!(competition.next_judge, Some(accounts.charlie));
            // ==== * it removes the current next_judge's CompetitionJudge
            assert!(az_trading_competition
                .competition_judges
                .get((0, accounts.django))
                .is_none());
            // ===== when current time is before or on current judge's deadline
            // ===== * it sets the next judge's deadline as 24 hours from the current judge's deadline
            let current_judge_deadline: Timestamp = az_trading_competition
                .competition_judges
                .get((0, competition.judge))
                .unwrap()
                .deadline;
            assert_eq!(
                az_trading_competition
                    .competition_judges
                    .get((0, accounts.charlie))
                    .unwrap()
                    .deadline,
                current_judge_deadline + DAY_IN_MS
            );
            // ===== when current time is after current judge's deadline
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                current_judge_deadline + 1,
            );
            competition.next_judge = None;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            az_trading_competition
                .competition_judges
                .remove((competition.id, accounts.charlie));
            // ==== * it creates a CompetitionJudge for caller with deadline set to 24 hours after current judge deadline or 24 hours into the future, whichever is greater
            az_trading_competition.next_judge_update(0).unwrap();
            assert_eq!(
                az_trading_competition
                    .competition_judges
                    .get((0, accounts.charlie))
                    .unwrap()
                    .deadline,
                current_judge_deadline + 1 + DAY_IN_MS
            );
        }

        #[ink::test]
        fn test_place_competitors() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.place_competitors(0, vec![]);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
            );
            // when competition exist
            let mut competition: Competition = az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                    None,
                    None,
                )
                .unwrap();
            let payout_structure = vec![(0, 5), (1, 4)];
            az_trading_competition
                .competition_payout_structure_numerators_update(
                    competition.id,
                    payout_structure.clone(),
                )
                .unwrap();
            competition = az_trading_competition
                .competitions
                .get(competition.id)
                .unwrap();
            // = when caller is not the competition's judge
            set_caller::<DefaultEnvironment>(accounts.charlie);
            // = * it raises an error
            let result = az_trading_competition.place_competitors(0, vec![]);
            assert_eq!(result, Err(AzTradingCompetitionError::Unauthorised));
            // = when caller is the competition's judge
            set_caller::<DefaultEnvironment>(accounts.bob);
            // == when competition hasn't ended
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(competition.end);
            // == * it raises an error
            let result = az_trading_competition.place_competitors(0, vec![]);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competition hasn't ended.".to_string(),
                ))
            );
            // == when competition has ended
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                competition.end + 1,
            );
            // === when all competitors have not had their final values set
            competition.competitors_count = 1;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // === * it raises an error
            let result = az_trading_competition.place_competitors(0, vec![]);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "All competitors have not had their final values updated.".to_string(),
                ))
            );
            // === when all competitors have had their final values set
            competition.competitor_final_value_updated_count = 1;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // ==== when any of the competitors are not part of the competition
            let result = az_trading_competition.place_competitors(0, vec![accounts.django]);
            // ==== * it raises an error
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competitor".to_string(),
                ))
            );
            // ==== when all competitors are part of the competition
            // ===== when any of the competitors have been placed in this placement round already
            let django_final_value: Option<String> = Some("5".to_string());
            az_trading_competition.competitors.insert(
                (competition.id, accounts.django),
                &Competitor {
                    final_value: django_final_value.clone(),
                    judge_place_attempt: 1,
                    competition_place_details_index: 0,
                },
            );
            // ===== * it raises an error
            let result = az_trading_competition.place_competitors(0, vec![accounts.django]);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competitor has already been placed.".to_string(),
                ))
            );
            // ===== when all of the competitors haven't been placed in this placement round
            az_trading_competition
                .competitors
                .remove((competition.id, accounts.django));
            az_trading_competition.competitors.insert(
                (competition.id, accounts.django),
                &Competitor {
                    final_value: django_final_value.clone(),
                    judge_place_attempt: 0,
                    competition_place_details_index: 0,
                },
            );
            // ====== when no competitors have been placed yet
            az_trading_competition
                .place_competitors(competition.id, vec![accounts.django])
                .unwrap();
            // ====== * it places the competitor in the first slot
            let mut competition_place_details_vec: Vec<CompetitionPlaceDetail> =
                az_trading_competition
                    .competition_place_details
                    .get(competition.id)
                    .unwrap();
            assert_eq!(
                competition_place_details_vec[0].competitor_value,
                django_final_value.clone().unwrap(),
            );
            // ====== * it sets the competitor count to 1
            assert_eq!(competition_place_details_vec[0].competitors_count, 1);
            // ====== * it sets the payout numerator for the first spot
            assert_eq!(
                competition_place_details_vec[0].payout_numerator,
                payout_structure[0].1
            );
            // ====== * it sets the competitor's competition_place_details_index
            assert_eq!(
                az_trading_competition
                    .competitors
                    .get((competition.id, accounts.django))
                    .unwrap()
                    .competition_place_details_index,
                0
            );
            // ====== when some competitors have been placed so far
            // ======= when competitor has the same final value as the last placed competitor
            az_trading_competition.competitors.insert(
                (competition.id, accounts.charlie),
                &Competitor {
                    final_value: django_final_value.clone(),
                    judge_place_attempt: 0,
                    competition_place_details_index: 0,
                },
            );
            az_trading_competition
                .place_competitors(competition.id, vec![accounts.charlie])
                .unwrap();
            competition_place_details_vec = az_trading_competition
                .competition_place_details
                .get(competition.id)
                .unwrap();
            // ======= * it adds to the latest place's count
            assert_eq!(competition_place_details_vec.len(), 1);
            assert_eq!(competition_place_details_vec[0].competitors_count, 2);
            // ====== * it adds to the latest place's numerator
            assert_eq!(
                competition_place_details_vec[0].payout_numerator,
                payout_structure[0].1 + payout_structure[1].1
            );
            // ====== * it sets the competitor's competition_place_details_index
            assert_eq!(
                az_trading_competition
                    .competitors
                    .get((competition.id, accounts.charlie))
                    .unwrap()
                    .competition_place_details_index,
                0
            );
            // ======= when competitor has a higher final value than the last placed competitor
            let bob_final_value: String = "6".to_string();
            az_trading_competition.competitors.insert(
                (competition.id, accounts.bob),
                &Competitor {
                    final_value: Some(bob_final_value.clone()),
                    judge_place_attempt: 0,
                    competition_place_details_index: 0,
                },
            );
            az_trading_competition
                .place_competitors(competition.id, vec![accounts.bob])
                .unwrap();
            // ======= * it places the competitor onto the end
            competition_place_details_vec = az_trading_competition
                .competition_place_details
                .get(competition.id)
                .unwrap();
            assert_eq!(competition_place_details_vec.len(), 2);
            assert_eq!(
                competition_place_details_vec[1].competitor_value,
                bob_final_value
            );
            assert_eq!(competition_place_details_vec[1].competitors_count, 1);
            // ======= * it sets the competitor's competition_place_details_index
            assert_eq!(
                az_trading_competition
                    .competitors
                    .get((competition.id, accounts.bob))
                    .unwrap()
                    .competition_place_details_index,
                1
            );
            // ====== * it sets the competitor count to 1
            assert_eq!(competition_place_details_vec[1].competitors_count, 1);
            // ====== * it sets the payout numerator for the second spot
            assert_eq!(competition_place_details_vec[1].payout_numerator, 0);
            // ======= when competitor has a lower final value than the last placed competitor
            az_trading_competition.competitors.insert(
                (competition.id, accounts.frank),
                &Competitor {
                    final_value: Some("0".to_string()),
                    judge_place_attempt: 0,
                    competition_place_details_index: 0,
                },
            );
            // ======= it raises an error
            let result =
                az_trading_competition.place_competitors(competition.id, vec![accounts.frank]);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Competitor is in the wrong place.".to_string(),
                ))
            );
            // ===== * it updates competitors' placement rounds
            assert_eq!(
                az_trading_competition
                    .competitors
                    .get((competition.id, accounts.django))
                    .unwrap()
                    .judge_place_attempt,
                1
            );
            assert_eq!(
                az_trading_competition
                    .competitors
                    .get((competition.id, accounts.charlie))
                    .unwrap()
                    .judge_place_attempt,
                1
            );
            assert_eq!(
                az_trading_competition
                    .competitors
                    .get((competition.id, accounts.bob))
                    .unwrap()
                    .judge_place_attempt,
                1
            );
            assert_eq!(
                az_trading_competition
                    .competitors
                    .get((competition.id, accounts.frank))
                    .unwrap()
                    .judge_place_attempt,
                0
            );
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
            // === when caller has registered already
            az_trading_competition.competition_token_competitors.insert(
                (0, mock_entry_fee_token(), accounts.bob),
                &CompetitionTokenCompetitor {
                    amount: MOCK_ENTRY_FEE_AMOUNT,
                    collected: false,
                },
            );
            // == * it raises an error
            let result = az_trading_competition.register(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Already registered".to_string(),
                ))
            );
            // === when caller has not registered yet
            az_trading_competition
                .competition_token_competitors
                .remove((0, mock_entry_fee_token(), accounts.bob));
            // === when azero_processing fee has not been sent
            // === * it raises an error
            let result = az_trading_competition.register(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Please include AZERO processing fee.".to_string(),
                ))
            );
            // === the rest needs to be done in integration tests
        }

        #[ink::test]
        fn test_reset() {
            let (accounts, mut az_trading_competition) = init();
            // when competition does not exist
            // * it raises an error
            let result = az_trading_competition.reset(0);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::NotFound(
                    "Competition".to_string(),
                ))
            );
            // when competition exist
            let mut competition: Competition = az_trading_competition
                .competitions_create(
                    MOCK_START,
                    MOCK_START + MINIMUM_DURATION,
                    mock_entry_fee_token(),
                    MOCK_ENTRY_FEE_AMOUNT,
                    None,
                    None,
                )
                .unwrap();
            // = when caller is not the judge of the competition
            set_caller::<DefaultEnvironment>(accounts.django);
            let result = az_trading_competition.reset(competition.id);
            // = * it raises an error
            assert_eq!(result, Err(AzTradingCompetitionError::Unauthorised));
            // = when caller is the judge of the competition
            set_caller::<DefaultEnvironment>(competition.judge);
            // == when judge_place_attempt has reached the maximum
            competition.judge_place_attempt = u128::MAX;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // == * it raises an error
            let result = az_trading_competition.reset(competition.id);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Maximum place attempts reached.".to_string(),
                ))
            );
            // == when judge_place_attempt has not reached the maximum.
            competition.judge_place_attempt -= 1;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // === when competition hasn't had any competitors placed yet
            // === * it raises an error
            let result = az_trading_competition.reset(competition.id);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "Zero competitors have been placed.".to_string(),
                ))
            );
            // === when competition has competitors placed
            competition.competitors_placed_count = 1;
            // ==== when all competitors have been placed
            competition.competitors_count = 1;
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            // ==== * it raises an error
            let result = az_trading_competition.reset(competition.id);
            assert_eq!(
                result,
                Err(AzTradingCompetitionError::UnprocessableEntity(
                    "All competitors have been placed.".to_string(),
                ))
            );
            // ===== when all competitors haven't been placed
            competition.competitors_count = 2;
            let mut competition_place_details_vec: Vec<CompetitionPlaceDetail> =
                az_trading_competition
                    .competition_place_details
                    .get(competition.id)
                    .unwrap();
            competition_place_details_vec.push(CompetitionPlaceDetail {
                competitor_value: "123".to_string(),
                competitors_count: 1,
                payout_numerator: 1,
            });
            az_trading_competition
                .competition_place_details
                .insert(competition.id, &competition_place_details_vec);
            az_trading_competition
                .competitions
                .insert(competition.id, &competition);
            az_trading_competition.reset(competition.id).unwrap();
            competition = az_trading_competition
                .competitions
                .get(competition.id)
                .unwrap();
            // ===== * it sets the competitors_placed_count to zero
            assert_eq!(competition.competitors_placed_count, 0);
            // ===== * it resets place_details_ordered_by_competitor_final_value
            competition_place_details_vec = az_trading_competition
                .competition_place_details
                .get(competition.id)
                .unwrap();
            assert_eq!(competition_place_details_vec.len(), 0);
            // ===== * it increases the judge_place_attempt by one
            assert_eq!(competition.judge_place_attempt, u128::MAX);
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
            // == when competitor count is less than payout places
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
            // == when competitor count is greater than or equal to payout places
            competition.competitors_count = competition.payout_places.into();
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
            // ==== when competitor is not present
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
                    "CompetitionTokenCompetitor".to_string(),
                ))
            );
            // ==== when competitor is present
            az_trading_competition.competition_token_competitors.insert(
                (id, path[0], accounts.bob),
                &CompetitionTokenCompetitor {
                    amount: 0,
                    collected: false,
                },
            );
            // ===== when amount_in is greater than what is available to competitor
            amount_in = az_trading_competition
                .competition_token_competitors_show(id, path[0], accounts.bob)
                .unwrap()
                .amount
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
            // ===== when amount_in is available to competitor
            amount_in = az_trading_competition
                .competition_token_competitors_show(id, path[0], accounts.bob)
                .unwrap()
                .amount;
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
