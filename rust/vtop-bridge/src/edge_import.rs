//! An account as the edge's `Account` object imports it
//! (`POST /internal/accounts/import`). Sealed fields move over as they are:
//! the engine seals with the same key and the same `accounts/<id>` binding.
//! Only the FCM token is opened, so the edge can make an install for it.

use serde_json::{json, Value};

use crate::store::{AccountDoc, KeyDoc};

/// The import body for `doc` and its keys. `session_cache_secs` turns the
/// bridge's session expiry back into when it was saved.
pub fn import_body(
    doc: &AccountDoc,
    keys: &[KeyDoc],
    fcm_token: Option<String>,
    session_cache_secs: u64,
) -> Value {
    let settings = doc.settings.unwrap_or_default();
    let vault_saved_at =
        doc.vault
            .as_ref()
            .map(|_| match (doc.vault_expires_at, settings.vault_ttl_secs) {
                (Some(expires), Some(ttl)) => expires.saturating_sub(ttl),
                _ => doc.updated_at,
            });
    let session_saved_at = doc.session.as_ref().map(|_| {
        doc.session_expires_at.map_or(doc.updated_at, |expires| {
            expires.saturating_sub(session_cache_secs)
        })
    });
    let keys: Vec<Value> = keys
        .iter()
        .map(|key| {
            json!({
                "hash": key.hash,
                "label": key.label,
                "created_at": key.created_at,
                "last_used_at": key.last_used_at,
            })
        })
        .collect();
    json!({
        "record": {
            "id": doc.username,
            "app_secret_hash": doc.app_secret_hash,
            "settings": settings,
            "semester_id": doc.semester_id,
            "sealed_vault": doc.vault,
            "vault_saved_at": vault_saved_at,
            "sealed_session": doc.session,
            "session_saved_at": session_saved_at,
            "keys": keys,
            "created_at": doc.updated_at,
            "updated_at": doc.updated_at,
        },
        "fcmToken": fcm_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtop_shared::account::AccountSettings;

    #[test]
    fn carries_sealed_fields_and_saved_times() {
        let mut doc = AccountDoc::empty("22BCE0001", 1_000_000);
        doc.vault = Some("sealed-vault".into());
        doc.vault_expires_at = Some(1_086_400);
        doc.session = Some("sealed-session".into());
        doc.session_expires_at = Some(1_604_800);
        doc.app_secret_hash = Some("h".into());
        doc.settings = Some(AccountSettings {
            vault_ttl_secs: Some(86_400),
            ..AccountSettings::default()
        });
        let key = KeyDoc {
            hash: "abc".into(),
            registration_number: "22BCE0001".into(),
            label: "Claude".into(),
            created_at: 5,
            last_used_at: None,
        };
        let body = import_body(&doc, &[key], Some("fcm".into()), 604_800);
        let record = &body["record"];
        assert_eq!(record["id"], "22BCE0001");
        assert_eq!(record["sealed_vault"], "sealed-vault");
        assert_eq!(record["vault_saved_at"], 1_000_000);
        assert_eq!(record["session_saved_at"], 1_000_000);
        assert_eq!(record["settings"]["vaultTtlSecs"], 86_400);
        assert_eq!(record["keys"][0]["label"], "Claude");
        assert_eq!(body["fcmToken"], "fcm");

        let bare = import_body(&AccountDoc::empty("X1", 7), &[], None, 604_800);
        assert!(bare["record"]["vault_saved_at"].is_null());
        assert!(bare["fcmToken"].is_null());
    }
}
