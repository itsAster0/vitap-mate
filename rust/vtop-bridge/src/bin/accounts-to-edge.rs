//! Moves accounts and their access keys from Firestore (where the old
//! bridge keeps them) into the edge's `Account` objects. The FCM token is
//! opened with VAULT_KEY so the edge can make an install for the phone;
//! nothing else is opened. Saved passwords are left behind: they were for
//! the Gmail sign-in, and server sign-in now needs email forwarding, so the
//! student turns it on again in the app.
//!
//! Reads FIREBASE_CREDENTIALS_JSON, EDGE_URL, MAIL_RELAY_SECRET and
//! VAULT_KEY. Without `--write` it only counts. `--overwrite` replaces
//! accounts already imported. Prints counts, never values.

use std::collections::BTreeSet;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use vtop_bridge::edge_import::import_body;
use vtop_bridge::firestore::{account_from_fields, list_documents, FirestoreStore, DEFAULT_BASE_URL};
use vtop_bridge::google::{ServiceAccountTokens, TokenSource};
use vtop_bridge::store::{Store, ACCOUNTS};
use vtop_bridge::vault::Sealer;

/// The engine's session cache; the bridge stored when sessions expire.
const SESSION_CACHE_SECS: u64 = 7 * 24 * 3600;

fn env(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            eprintln!("accounts-to-edge: {name} is required");
            std::process::exit(2);
        })
}

async fn run(write: bool, overwrite: bool, extra: Vec<String>) -> Result<(), String> {
    let tokens: Arc<dyn TokenSource> = Arc::new(ServiceAccountTokens::from_json(&env(
        "FIREBASE_CREDENTIALS_JSON",
    ))?);
    let key: [u8; 32] = STANDARD
        .decode(env("VAULT_KEY").trim())
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or("VAULT_KEY is not 32 base64 bytes")?;
    let sealer = Sealer::new(key);
    let http = reqwest::Client::new();
    let edge_url = env("EDGE_URL");
    let secret = env("MAIL_RELAY_SECRET");
    let store = FirestoreStore::new(http.clone(), tokens.clone(), DEFAULT_BASE_URL.into());

    let firestore = list_documents(&http, &tokens, ACCOUNTS).await?;
    let mut ids: BTreeSet<String> = firestore.iter().map(|(id, _)| id.clone()).collect();
    ids.extend(extra.iter().map(|id| id.trim().to_uppercase()));
    println!("{} accounts to move", ids.len());

    let (mut moved, mut existing, mut with_phone, mut keys_total) = (0, 0, 0, 0);
    for id in &ids {
        let doc = match firestore.iter().find(|(fid, _)| fid == id) {
            Some((fid, fields)) => account_from_fields(fid, fields),
            None => match store.get_account(id).await.map_err(|e| e.to_string())? {
                Some(doc) => doc,
                None => {
                    println!("one account was not found; skipped");
                    continue;
                }
            },
        };
        let keys = store.list_keys(id).await.map_err(|e| e.to_string())?;
        let fcm_token = doc.fcm_token.as_deref().and_then(|sealed| {
            sealer
                .open_json::<String>(&format!("{ACCOUNTS}/{id}"), sealed)
                .ok()
        });
        keys_total += keys.len();
        with_phone += usize::from(fcm_token.is_some());
        if !write {
            continue;
        }
        let mut body = import_body(&doc, &keys, fcm_token, SESSION_CACHE_SECS);
        body["record"]["sealed_vault"] = serde_json::Value::Null;
        body["record"]["vault_saved_at"] = serde_json::Value::Null;
        body["overwrite"] = overwrite.into();
        let response = http
            .post(format!("{edge_url}/internal/accounts/import"))
            .bearer_auth(&secret)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.without_url().to_string())?;
        match response.status().as_u16() {
            201 => moved += 1,
            409 => existing += 1,
            status => return Err(format!("import answered {status}")),
        }
    }
    println!("{keys_total} access keys, {with_phone} accounts with a phone to reach");
    if write {
        println!("imported {moved}, already there {existing}");
    } else {
        println!("dry run; pass --write to import");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let write = args.iter().any(|arg| arg == "--write");
    let overwrite = args.iter().any(|arg| arg == "--overwrite");
    let extra = args
        .windows(2)
        .filter(|pair| pair[0] == "--account")
        .map(|pair| pair[1].clone())
        .collect();
    if let Err(error) = run(write, overwrite, extra).await {
        eprintln!("accounts-to-edge: {error}");
        std::process::exit(1);
    }
}
