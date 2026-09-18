//! Wall-clock helper. Unix epoch milliseconds — matches the integer timestamp
//! columns in the schema and `Date.now()` on the React side.

use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_ms_is_after_2020() {
        assert!(
            now_ms() > 1_577_836_800_000,
            "should be well after 2020-01-01"
        );
    }
}
