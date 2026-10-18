//! Firebase Cloud Messaging from the Worker. The OAuth token is minted from
//! the service account with WebCrypto (RS256) and cached for its hour in
//! the `FcmToken` Durable Object, so pushes don't each pay for a signature.

use std::cell::RefCell;

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use serde::Deserialize;
use serde_json::json;
use wasm_bindgen::{JsCast, JsValue};
use worker::{
    durable_object, DurableObject, Env, Fetch, Headers, Method, Request, RequestInit, Response,
    Result, State,
};

/// An OAuth scope name, not an address.
const SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";
/// Mint a new token this long before the old one expires.
const REFRESH_MARGIN_SECS: u64 = 300;

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccount {
    pub client_email: String,
    pub private_key: String,
    pub project_id: String,
    /// Where the assertion is exchanged; part of every service account file.
    pub token_uri: String,
}

/// `header.claims` of the OAuth assertion, base64url.
pub fn unsigned_jwt(account: &ServiceAccount, now: u64) -> String {
    let header = json!({ "alg": "RS256", "typ": "JWT" });
    let claims = json!({
        "iss": account.client_email,
        "scope": SCOPE,
        "aud": account.token_uri,
        "iat": now,
        "exp": now + 3600,
    });
    format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header.to_string()),
        URL_SAFE_NO_PAD.encode(claims.to_string())
    )
}

/// The DER bytes inside a PEM `PRIVATE KEY` block.
pub fn pem_to_der(pem: &str) -> Option<Vec<u8>> {
    let body: String = pem
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("-----"))
        .collect();
    STANDARD.decode(body).ok()
}

/// The data map FCM carries: string values only.
pub fn message(
    token: &str,
    data: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let data: serde_json::Map<String, serde_json::Value> = data
        .iter()
        .map(|(key, value)| {
            let text = match value {
                serde_json::Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            (key.clone(), serde_json::Value::String(text))
        })
        .collect();
    json!({ "message": {
        "token": token,
        "data": data,
        "android": { "priority": "HIGH", "ttl": "120s" },
    }})
}

fn js_error(context: &str, error: JsValue) -> worker::Error {
    worker::Error::RustError(format!("{context}: {error:?}"))
}

async fn call(target: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue> {
    let function: js_sys::Function = js_sys::Reflect::get(target, &method.into())
        .map_err(|e| js_error(method, e))?
        .dyn_into()
        .map_err(|e| js_error(method, e))?;
    let promise: js_sys::Promise = function
        .apply(target, &args.iter().collect::<js_sys::Array>())
        .map_err(|e| js_error(method, e))?
        .dyn_into()
        .map_err(|e| js_error(method, e))?;
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| js_error(method, e))
}

/// RS256 signature with WebCrypto, base64url.
async fn sign_rs256(der: &[u8], data: &str) -> Result<String> {
    let crypto = js_sys::Reflect::get(&js_sys::global(), &"crypto".into())
        .map_err(|e| js_error("crypto", e))?;
    let subtle =
        js_sys::Reflect::get(&crypto, &"subtle".into()).map_err(|e| js_error("subtle", e))?;
    let algorithm = js_sys::Object::new();
    js_sys::Reflect::set(&algorithm, &"name".into(), &"RSASSA-PKCS1-v1_5".into())
        .map_err(|e| js_error("algorithm", e))?;
    let hash = js_sys::Object::new();
    js_sys::Reflect::set(&hash, &"name".into(), &"SHA-256".into())
        .map_err(|e| js_error("hash", e))?;
    js_sys::Reflect::set(&algorithm, &"hash".into(), &hash)
        .map_err(|e| js_error("algorithm", e))?;
    let usages = js_sys::Array::of1(&"sign".into());
    let key = call(
        &subtle,
        "importKey",
        &[
            "pkcs8".into(),
            js_sys::Uint8Array::from(der).buffer().into(),
            algorithm.clone().into(),
            JsValue::FALSE,
            usages.into(),
        ],
    )
    .await?;
    let signature = call(
        &subtle,
        "sign",
        &[
            algorithm.into(),
            key,
            js_sys::Uint8Array::from(data.as_bytes()).buffer().into(),
        ],
    )
    .await?;
    let bytes = js_sys::Uint8Array::new(&signature).to_vec();
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn service_account(env: &Env) -> Result<ServiceAccount> {
    let raw = env.secret("FIREBASE_CREDENTIALS_JSON")?.to_string();
    serde_json::from_str(&raw)
        .map_err(|e| worker::Error::RustError(format!("service account: {e}")))
}

async fn mint(account: &ServiceAccount, now: u64) -> Result<(String, u64)> {
    let der = pem_to_der(&account.private_key)
        .ok_or_else(|| worker::Error::RustError("service account key is not PEM".into()))?;
    let unsigned = unsigned_jwt(account, now);
    let jwt = format!("{unsigned}.{}", sign_rs256(&der, &unsigned).await?);
    let headers = Headers::new();
    headers.set("content-type", "application/x-www-form-urlencoded")?;
    let body =
        format!("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion={jwt}");
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body.into()));
    let mut response = Fetch::Request(Request::new_with_init(account.token_uri.as_str(), &init)?)
        .send()
        .await?;
    if response.status_code() != 200 {
        return Err(worker::Error::RustError(format!(
            "token exchange failed: {}",
            response.status_code()
        )));
    }
    #[derive(Deserialize)]
    struct Token {
        access_token: String,
        expires_in: u64,
    }
    let token: Token = response.json().await?;
    Ok((token.access_token, now + token.expires_in))
}

/// Caches the FCM OAuth token. One instance, named `token`.
#[durable_object]
pub struct FcmToken {
    env: Env,
    cached: RefCell<Option<(String, u64)>>,
}

impl DurableObject for FcmToken {
    fn new(_state: State, env: Env) -> Self {
        Self {
            env,
            cached: RefCell::new(None),
        }
    }

    async fn fetch(&self, _req: Request) -> Result<Response> {
        let now = crate::now();
        let cached = self.cached.borrow().clone();
        let token = match cached {
            Some((token, expires)) if expires > now + REFRESH_MARGIN_SECS => token,
            _ => {
                let (token, expires) = mint(&service_account(&self.env)?, now).await?;
                *self.cached.borrow_mut() = Some((token.clone(), expires));
                token
            }
        };
        Response::ok(token)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SendOutcome {
    Sent,
    /// The token no longer reaches an install.
    TokenInvalid,
    Failed,
}

/// Sends a high-priority data message.
pub async fn send(
    env: &Env,
    token: &str,
    data: &serde_json::Map<String, serde_json::Value>,
) -> Result<SendOutcome> {
    let account = service_account(env)?;
    let stub = env.durable_object("FCM_TOKEN")?.get_by_name("token")?;
    let mut token_response = stub.fetch_with_str("https://do/token").await?;
    if token_response.status_code() != 200 {
        return Ok(SendOutcome::Failed);
    }
    let access = token_response.text().await?;
    let headers = Headers::new();
    headers.set("authorization", &format!("Bearer {access}"))?;
    headers.set("content-type", "application/json")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(message(token, data).to_string().into()));
    let url = format!(
        "{}/v1/projects/{}/messages:send",
        crate::var(env, "FCM_URL")?,
        account.project_id
    );
    let mut response = Fetch::Request(Request::new_with_init(&url, &init)?)
        .send()
        .await?;
    Ok(match response.status_code() {
        200 => SendOutcome::Sent,
        404 => SendOutcome::TokenInvalid,
        400 if response
            .text()
            .await
            .unwrap_or_default()
            .contains("UNREGISTERED") =>
        {
            SendOutcome::TokenInvalid
        }
        _ => SendOutcome::Failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> ServiceAccount {
        ServiceAccount {
            client_email: "push@p.iam.gserviceaccount.com".into(),
            private_key: "-----BEGIN PRIVATE KEY-----\nAAEC\nAw==\n-----END PRIVATE KEY-----\n"
                .into(),
            project_id: "p".into(),
            token_uri: "https://oauth2.googleapis.com/token".into(),
        }
    }

    #[test]
    fn assertion_claims() {
        let jwt = unsigned_jwt(&account(), 100);
        let (header, claims) = jwt.split_once('.').unwrap();
        let header: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(header).unwrap()).unwrap();
        let claims: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims).unwrap()).unwrap();
        assert_eq!(header["alg"], "RS256");
        assert_eq!(claims["iss"], "push@p.iam.gserviceaccount.com");
        assert_eq!(claims["aud"], "https://oauth2.googleapis.com/token");
        assert_eq!(claims["scope"], SCOPE);
        assert_eq!(claims["exp"], 3700);
    }

    #[test]
    fn pem_body_is_decoded() {
        assert_eq!(pem_to_der(&account().private_key), Some(vec![0, 1, 2, 3]));
    }

    #[test]
    fn data_values_become_strings() {
        let mut data = serde_json::Map::new();
        data.insert("type".into(), "mail_otp".into());
        data.insert("seq".into(), 7.into());
        let body = message("tok", &data);
        assert_eq!(
            body["message"]["data"],
            json!({"type": "mail_otp", "seq": "7"})
        );
        assert_eq!(body["message"]["android"]["priority"], "HIGH");
    }
}
