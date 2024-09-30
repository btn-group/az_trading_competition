- There's a possible u32::max number of users
- If there's a tie how to slip up the prize between a possible 4,294,967,295 people.
- Unlikely scenario but possible and should be accounted for.
- I think the only way is early bird gets the worm.
- Should the numerator be tracked, or the prize amount or both?
- I don't know if it should even have a calculation_factor... The calculation facor was important for a token distibutor sort of thing, but this is 

Worst case scenario:

4,294,967,295 people are tied for 1 spot
denominator is 10_000
considering 1 azero is 1_000_000_000_000


- numerator_for_price * amount_of_tokens / total_denominator / number_of_users_for_price
- 10_000 * 1_000_000_000_000 * 1_000_000_000_000_000_000 / 10_000 / 4_294_967_295 = 232
- So there will be left overs... I think we're going to have to employ a calculation factor as usual
-



amount_for_user = numerator_for_users * amount_of_tokens / denominator / number_of_users
- It still ends up being 0 for the user
- Yeah I don't think we need a calculation_factor
- The calculation factor was important for calculating token 
- So in summary I don't think I need a calculation scale.
- What I do need though is to keep a token prize collected, so that if somebody comes to collect and what they're entitled to is larger than what's available, they can take whatever is left over.