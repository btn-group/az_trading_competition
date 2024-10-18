// When can this be called?
// After a certain amount of time for sure.
// What if some users have been processed?

// Do tokens individually as the concern is running out of block space:
// If your biggest concern is it gassing out, you gotta be able to do it individually.
#[ink(message)]
pub fn emergency_rescue(&mut self, id: u64, token: AccountId) -> Result<()> {
    // 1. Get competition
    // 2. If funds are still available for user after a month, transfer it back to user.
    // 3. Set balance to zero.

    Ok(())
}

// - When happens in an emergency rescue situation with the fees from the failed judges?
// - I think easiest way to go about it is to send it to the admin.
