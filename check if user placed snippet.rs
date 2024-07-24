            // 8. Check if user has placed
            // Gotta store the top position places
            // The whole efficiency is not that important considering I'm only concerned about it timing out...
            // The only thing that matters than, is how to sort an array that could technically be the size of the full user_count...
            // idea
            // let winning_prices: Vec<Vec<(String, user_count)>> = vec![];
            // let _payout_places_accounted_for: u32 = 0;
            // let mut users_accounted_for: u32 = 0;
            for (index, payout_winning_price_and_user_count) in competition
                .payout_winning_price_and_user_counts
                .iter()
                .enumerate()
            {
                if payout_winning_price_and_user_count.0 == user_usd_value_as_string {
                    let new_user_count: u32 = payout_winning_price_and_user_count.1 + 1;
                    competition
                        .payout_winning_price_and_user_counts
                        .insert(index, (user_usd_value_as_string.clone(), new_user_count));
                }
            }

            // 9. Send percentage of profits to user (If there's more tokens, may have to create a separate function for this)

            Ok(())
