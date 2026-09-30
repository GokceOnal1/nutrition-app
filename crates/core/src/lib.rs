//! Pure domain and analytics logic for the nutrition/training tracker.
//!
//! This crate must stay free of I/O, async, `sqlx`, and `axum`. Every
//! derived number shown to the user (trend weight, TDEE, macro totals,
//! training load) is computed here by a pure function that is unit-tested
//! without a database. Domain types and analytics land in later milestones;
//! see `CLAUDE.md`.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
