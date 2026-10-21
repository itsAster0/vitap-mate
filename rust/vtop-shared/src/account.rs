//! Account ids, as the engine and the edge
//! (`vtop-edge/src/shared/engine-api.ts`) both make them.

/// Account ids are upper-cased registration numbers.
pub fn account_id(registration_number: &str) -> String {
    registration_number.trim().to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_trimmed_and_upper_cased() {
        assert_eq!(account_id(" 23abc4567 "), "23ABC4567");
    }
}
