# Emergency Rescue

## When to allow emergency_rescue

1. When judge_place_attempt has maxxed out.
- This should be allowed when when judge attempts has maxxed out
- This means that you shouldn’t let the judge place users when this has maxxed out…

2. When a certain amount of time has elapsed.
- Considering judge_update costs a fee, it's possible that the amount of tokens will run out before judge_place_attempt reaches the maximum limit, so time limit should be set as well.
- In the case, should it always be a time limit?

## Handling fees from failed judges

- I think easiest way to go about it is to send it to the admin.
- Will have to track amount received in judge fees.

## Limiting the number of times a judge can call reset

- Reset can be called an infinite numer of times by one person.
- Setting a limit of 10 to stop this.
