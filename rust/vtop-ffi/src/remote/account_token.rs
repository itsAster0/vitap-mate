//! The v2 edge's account token (`backend/API.md`, "Account tokens"), which a phone that proved
//! before v3 still keeps: traded in once for a refresh token (`auth:token { v2AccountToken }`,
//! [`crate::remote::backend`]), then dropped. Kept in [`FILE`] for the signed-in student;
//! dropped on sign-out and another student too. The token itself is never logged.

use std::fs;

use serde::{Deserialize, Serialize};

use crate::Core;

pub(crate) const FILE: &str = "account_token.json";

/// No `Debug`: it holds the token.
#[derive(Serialize, Deserialize)]
pub(crate) struct Saved {
    pub(crate) server: String,
    pub(crate) registration_number: String,
    pub(crate) token: String,
    /// Unix ms.
    pub(crate) expires_at: i64,
}

impl Core {
    pub(crate) fn forget_account_token(&self) {
        match fs::remove_file(self.at(FILE)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                tracing::warn!(%error, "account token not removed");
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_out_drops_the_token() {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-token-out-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let core = crate::test_edge::signed_in(&dir, "http://127.0.0.1:9".into());
        let saved = Saved {
            server: "https://vmate.aster0.dev".into(),
            registration_number: "22BCE0001".into(),
            token: "tok".into(),
            expires_at: 0,
        };
        core.write_plain(FILE, &saved);
        core.sign_out("f".into());
        assert!(!core.at(FILE).exists());
    }
}
