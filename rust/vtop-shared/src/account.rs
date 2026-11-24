//! Account ids, as the engine and the edge
//! (`backend/convex/lib/engineApi.ts`) both make them.

/// Account ids are upper-cased registration numbers.
pub fn account_id(registration_number: &str) -> String {
    registration_number.trim().to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_trimmed_and_upper_cased() {
        // Shared with the edge's `tokens.test.ts`.
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("../vectors/tokens.json")).unwrap();
        for v in vectors["accountIds"].as_array().unwrap() {
            assert_eq!(
                account_id(v["registrationNumber"].as_str().unwrap()),
                v["id"]
            );
        }
    }
}
