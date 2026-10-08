//! [`Store`] over the Firestore REST API.
//!
//! Timestamps are `timestampValue`s so Firestore's TTL policy can expire
//! `bridge_requests` on `expiresAt`. Completing a request is a
//! compare-and-set on the document's `updateTime`.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Map, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::google::TokenSource;
use crate::store::{
    apply_completion, AccountDoc, CompleteError, Completion, KeyDoc, RequestDoc, RequestStatus,
    Store, StoreError, ACCESS_KEYS, ACCOUNTS, FCM_INDEX, REQUESTS,
};

pub const DEFAULT_BASE_URL: &str = "https://firestore.googleapis.com/v1";
const CALL_TIMEOUT: Duration = Duration::from_secs(5);

pub struct FirestoreStore {
    http: reqwest::Client,
    tokens: Arc<dyn TokenSource>,
    base_url: String,
}

// ---- value encoding -------------------------------------------------------

fn timestamp(secs: u64) -> Value {
    let formatted = i64::try_from(secs)
        .ok()
        .and_then(|secs| OffsetDateTime::from_unix_timestamp(secs).ok())
        .and_then(|time| time.format(&Rfc3339).ok())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".into());
    json!({ "timestampValue": formatted })
}

fn string(value: &str) -> Value {
    json!({ "stringValue": value })
}

fn read_timestamp(fields: &Map<String, Value>, name: &str) -> Option<u64> {
    let raw = fields.get(name)?.get("timestampValue")?.as_str()?;
    let time = OffsetDateTime::parse(raw, &Rfc3339).ok()?;
    u64::try_from(time.unix_timestamp()).ok()
}

fn read_string(fields: &Map<String, Value>, name: &str) -> Option<String> {
    fields
        .get(name)?
        .get("stringValue")?
        .as_str()
        .map(String::from)
}

fn read_bool(fields: &Map<String, Value>, name: &str) -> bool {
    fields
        .get(name)
        .and_then(|value| value.get("booleanValue"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn put_opt(fields: &mut Map<String, Value>, name: &str, value: &Option<String>) {
    if let Some(value) = value {
        fields.insert(name.into(), string(value));
    }
}

pub fn request_fields(doc: &RequestDoc) -> Value {
    let mut fields = Map::new();
    put_opt(&mut fields, "responseToken", &doc.response_token);
    fields.insert("status".into(), string(doc.status.as_str()));
    put_opt(&mut fields, "cookies", &doc.cookies_json);
    put_opt(&mut fields, "error", &doc.error);
    put_opt(&mut fields, "fcmHash", &doc.fcm_hash);
    put_opt(&mut fields, "registrationNumber", &doc.account);
    fields.insert(
        "wantCredentials".into(),
        json!({ "booleanValue": doc.want_credentials }),
    );
    fields.insert("createdAt".into(), timestamp(doc.created_at));
    fields.insert("expiresAt".into(), timestamp(doc.expires_at));
    Value::Object(fields)
}

pub fn request_from_fields(id: &str, fields: &Map<String, Value>) -> Option<RequestDoc> {
    Some(RequestDoc {
        id: id.to_string(),
        response_token: read_string(fields, "responseToken"),
        status: RequestStatus::parse(&read_string(fields, "status")?)?,
        cookies_json: read_string(fields, "cookies"),
        error: read_string(fields, "error"),
        fcm_hash: read_string(fields, "fcmHash"),
        want_credentials: read_bool(fields, "wantCredentials"),
        account: read_string(fields, "registrationNumber"),
        created_at: read_timestamp(fields, "createdAt")?,
        expires_at: read_timestamp(fields, "expiresAt")?,
    })
}

pub fn account_fields(doc: &AccountDoc) -> Value {
    let mut fields = Map::new();
    put_opt(&mut fields, "vault", &doc.vault);
    put_opt(&mut fields, "session", &doc.session);
    if let Some(expires_at) = doc.session_expires_at {
        fields.insert("sessionExpiresAt".into(), timestamp(expires_at));
    }
    put_opt(&mut fields, "fcmToken", &doc.fcm_token);
    put_opt(&mut fields, "appSecretHash", &doc.app_secret_hash);
    if let Some(expires_at) = doc.vault_expires_at {
        fields.insert("vaultExpiresAt".into(), timestamp(expires_at));
    }
    let settings = doc
        .settings
        .map(|settings| serde_json::to_string(&settings).expect("settings serialise"));
    put_opt(&mut fields, "settings", &settings);
    fields.insert("updatedAt".into(), timestamp(doc.updated_at));
    Value::Object(fields)
}

pub fn account_from_fields(username: &str, fields: &Map<String, Value>) -> AccountDoc {
    AccountDoc {
        username: username.to_string(),
        vault: read_string(fields, "vault"),
        session: read_string(fields, "session"),
        session_expires_at: read_timestamp(fields, "sessionExpiresAt"),
        fcm_token: read_string(fields, "fcmToken"),
        app_secret_hash: read_string(fields, "appSecretHash"),
        vault_expires_at: read_timestamp(fields, "vaultExpiresAt"),
        settings: read_string(fields, "settings").and_then(|json| serde_json::from_str(&json).ok()),
        updated_at: read_timestamp(fields, "updatedAt").unwrap_or(0),
    }
}

pub fn key_fields(doc: &KeyDoc) -> Value {
    let mut fields = Map::new();
    fields.insert(
        "registrationNumber".into(),
        string(&doc.registration_number),
    );
    fields.insert("label".into(), string(&doc.label));
    fields.insert("createdAt".into(), timestamp(doc.created_at));
    if let Some(last_used_at) = doc.last_used_at {
        fields.insert("lastUsedAt".into(), timestamp(last_used_at));
    }
    Value::Object(fields)
}

pub fn key_from_fields(hash: &str, fields: &Map<String, Value>) -> Option<KeyDoc> {
    Some(KeyDoc {
        hash: hash.to_string(),
        registration_number: read_string(fields, "registrationNumber")?,
        label: read_string(fields, "label").unwrap_or_default(),
        created_at: read_timestamp(fields, "createdAt").unwrap_or(0),
        last_used_at: read_timestamp(fields, "lastUsedAt"),
    })
}

// ---- REST -----------------------------------------------------------------

fn store_error(context: &str, error: impl std::fmt::Display) -> StoreError {
    StoreError(format!("firestore {context}: {error}"))
}

/// A fetched document: its fields and `updateTime`.
struct Fetched {
    fields: Map<String, Value>,
    update_time: String,
}

impl FirestoreStore {
    pub fn new(http: reqwest::Client, tokens: Arc<dyn TokenSource>, base_url: String) -> Self {
        Self {
            http,
            tokens,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    fn collection_url(&self, collection: &str) -> String {
        format!(
            "{}/projects/{}/databases/(default)/documents/{collection}",
            self.base_url,
            self.tokens.project_id()
        )
    }

    fn doc_url(&self, collection: &str, id: &str) -> String {
        format!("{}/{}", self.collection_url(collection), urlencode(id))
    }

    async fn send(
        &self,
        context: &str,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, StoreError> {
        let token = self.tokens.token().await?;
        builder
            .bearer_auth(token)
            .timeout(CALL_TIMEOUT)
            .send()
            .await
            .map_err(|error| store_error(context, error.without_url()))
    }

    async fn fetch(&self, collection: &str, id: &str) -> Result<Option<Fetched>, StoreError> {
        let response = self
            .send("get", self.http.get(self.doc_url(collection, id)))
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(store_error("get", response.status()));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|error| store_error("get", error))?;
        let fields = body
            .get("fields")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let update_time = body
            .get("updateTime")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok(Some(Fetched {
            fields,
            update_time,
        }))
    }

    async fn patch(&self, collection: &str, id: &str, fields: Value) -> Result<(), StoreError> {
        let response = self
            .send(
                "patch",
                self.http
                    .patch(self.doc_url(collection, id))
                    .json(&json!({ "fields": fields })),
            )
            .await?;
        if !response.status().is_success() {
            return Err(store_error("patch", response.status()));
        }
        Ok(())
    }
}

impl FirestoreStore {
    async fn delete_doc(&self, collection: &str, id: &str) -> Result<(), StoreError> {
        let response = self
            .send("delete", self.http.delete(self.doc_url(collection, id)))
            .await?;
        if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        Err(store_error("delete", response.status()))
    }

    /// Documents of `collection` whose `registrationNumber` equals
    /// `registration_number`, as (id, fields).
    async fn query_by_account(
        &self,
        collection: &str,
        registration_number: &str,
    ) -> Result<Vec<(String, Map<String, Value>)>, StoreError> {
        let url = format!(
            "{}/projects/{}/databases/(default)/documents:runQuery",
            self.base_url,
            self.tokens.project_id()
        );
        let query = json!({ "structuredQuery": {
            "from": [{ "collectionId": collection }],
            "where": { "fieldFilter": {
                "field": { "fieldPath": "registrationNumber" },
                "op": "EQUAL",
                "value": { "stringValue": registration_number } } }
        } });
        let response = self.send("query", self.http.post(url).json(&query)).await?;
        if !response.status().is_success() {
            return Err(store_error("query", response.status()));
        }
        let rows: Vec<Value> = response
            .json()
            .await
            .map_err(|error| store_error("query", error))?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let document = row.get("document")?;
                let id = document
                    .get("name")?
                    .as_str()?
                    .rsplit('/')
                    .next()?
                    .to_string();
                let fields = document
                    .get("fields")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                Some((id, fields))
            })
            .collect())
    }
}

fn urlencode(id: &str) -> String {
    id.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[async_trait]
impl Store for FirestoreStore {
    async fn create_request(&self, doc: &RequestDoc) -> Result<(), StoreError> {
        let builder = self
            .http
            .post(self.collection_url(REQUESTS))
            .query(&[("documentId", doc.id.as_str())])
            .json(&json!({ "fields": request_fields(doc) }));
        let response = self.send("create", builder).await?;
        if !response.status().is_success() {
            return Err(store_error("create", response.status()));
        }
        Ok(())
    }

    async fn get_request(&self, id: &str) -> Result<Option<RequestDoc>, StoreError> {
        match self.fetch(REQUESTS, id).await? {
            None => Ok(None),
            Some(fetched) => request_from_fields(id, &fetched.fields)
                .map(Some)
                .ok_or_else(|| store_error("get", "malformed request document")),
        }
    }

    async fn complete_request(
        &self,
        id: &str,
        token: &str,
        completion: Completion,
        now: u64,
    ) -> Result<RequestDoc, CompleteError> {
        let fetched = self
            .fetch(REQUESTS, id)
            .await
            .map_err(CompleteError::Store)?
            .ok_or(CompleteError::NotFound)?;
        let mut doc = request_from_fields(id, &fetched.fields).ok_or_else(|| {
            CompleteError::Store(store_error("complete", "malformed request document"))
        })?;
        apply_completion(&mut doc, token, completion, now)?;

        // Every field in the mask but absent from the body is deleted, which
        // is how responseToken (and an unset cookies/error) goes away.
        let mask = ["status", "cookies", "error", "responseToken"];
        let mut query: Vec<(&str, &str)> = mask
            .iter()
            .map(|field| ("updateMask.fieldPaths", *field))
            .collect();
        query.push(("currentDocument.updateTime", fetched.update_time.as_str()));
        let builder = self
            .http
            .patch(self.doc_url(REQUESTS, id))
            .query(&query)
            .json(&json!({ "fields": request_fields(&doc) }));
        let response = self
            .send("complete", builder)
            .await
            .map_err(CompleteError::Store)?;
        let status = response.status();
        if status.is_success() {
            return Ok(doc);
        }
        let body: Value = response.json().await.unwrap_or(Value::Null);
        let reason = body
            .pointer("/error/status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if reason == "FAILED_PRECONDITION" || status == reqwest::StatusCode::CONFLICT {
            return Err(CompleteError::AlreadyHandled);
        }
        Err(CompleteError::Store(store_error("complete", status)))
    }

    async fn delete_request(&self, id: &str) -> Result<(), StoreError> {
        let response = self
            .send("delete", self.http.delete(self.doc_url(REQUESTS, id)))
            .await?;
        if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        Err(store_error("delete", response.status()))
    }

    async fn get_account(&self, username: &str) -> Result<Option<AccountDoc>, StoreError> {
        Ok(self
            .fetch(ACCOUNTS, username)
            .await?
            .map(|fetched| account_from_fields(username, &fetched.fields)))
    }

    async fn put_account(&self, doc: &AccountDoc) -> Result<(), StoreError> {
        self.patch(ACCOUNTS, &doc.username, account_fields(doc))
            .await
    }

    async fn username_for_fcm(&self, fcm_hash: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .fetch(FCM_INDEX, fcm_hash)
            .await?
            .and_then(|fetched| read_string(&fetched.fields, "username")))
    }

    async fn put_fcm_index(&self, fcm_hash: &str, username: &str) -> Result<(), StoreError> {
        self.patch(FCM_INDEX, fcm_hash, json!({ "username": string(username) }))
            .await
    }

    async fn put_key(&self, doc: &KeyDoc) -> Result<(), StoreError> {
        self.patch(ACCESS_KEYS, &doc.hash, key_fields(doc)).await
    }

    async fn get_key(&self, hash: &str) -> Result<Option<KeyDoc>, StoreError> {
        match self.fetch(ACCESS_KEYS, hash).await? {
            None => Ok(None),
            Some(fetched) => Ok(key_from_fields(hash, &fetched.fields)),
        }
    }

    async fn list_keys(&self, registration_number: &str) -> Result<Vec<KeyDoc>, StoreError> {
        Ok(self
            .query_by_account(ACCESS_KEYS, registration_number)
            .await?
            .into_iter()
            .filter_map(|(id, fields)| key_from_fields(&id, &fields))
            .collect())
    }

    async fn delete_key(&self, hash: &str) -> Result<(), StoreError> {
        self.delete_doc(ACCESS_KEYS, hash).await
    }

    async fn delete_account(&self, registration_number: &str) -> Result<(), StoreError> {
        for (id, _) in self
            .query_by_account(ACCESS_KEYS, registration_number)
            .await?
        {
            self.delete_doc(ACCESS_KEYS, &id).await?;
        }
        for (id, _) in self.query_by_account(REQUESTS, registration_number).await? {
            self.delete_doc(REQUESTS, &id).await?;
        }
        self.delete_doc(ACCOUNTS, registration_number).await
    }

    async fn health(&self) -> Result<(), StoreError> {
        let builder = self
            .http
            .get(self.collection_url(REQUESTS))
            .query(&[("pageSize", "1")]);
        let response = self.send("health", builder).await?;
        if !response.status().is_success() {
            return Err(store_error("health", response.status()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::google::StaticTokens;
    use wiremock::matchers::{body_partial_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const DOCS: &str = "/projects/p/databases/(default)/documents";

    fn request() -> RequestDoc {
        RequestDoc {
            id: "r1".into(),
            response_token: Some("tok".into()),
            status: RequestStatus::Pending,
            cookies_json: None,
            error: None,
            fcm_hash: Some("abc".into()),
            want_credentials: true,
            account: Some("22BCE0001".into()),
            created_at: 1_790_000_000,
            expires_at: 1_790_000_120,
        }
    }

    fn doc_body(name: &str, fields: Value, update_time: &str) -> Value {
        json!({ "name": format!("projects/p/databases/(default)/documents/{name}"),
                "fields": fields, "updateTime": update_time })
    }

    async fn store(server: &MockServer) -> FirestoreStore {
        FirestoreStore::new(reqwest::Client::new(), StaticTokens::arc("p"), server.uri())
    }

    #[test]
    fn request_doc_encodes_and_decodes() {
        let doc = request();
        let fields = request_fields(&doc);
        assert_eq!(
            fields["expiresAt"],
            json!({ "timestampValue": "2026-09-21T14:15:20Z" })
        );
        assert_eq!(fields["wantCredentials"], json!({ "booleanValue": true }));
        assert_eq!(fields["status"], json!({ "stringValue": "pending" }));
        assert!(fields.get("cookies").is_none());
        let back = request_from_fields("r1", fields.as_object().unwrap()).unwrap();
        assert!(back == doc);
    }

    #[test]
    fn timestamps_with_fractions_decode() {
        let mut fields = request_fields(&request()).as_object().unwrap().clone();
        fields.insert(
            "expiresAt".into(),
            json!({ "timestampValue": "2026-09-21T14:15:20.123456Z" }),
        );
        assert_eq!(
            request_from_fields("r1", &fields).unwrap().expires_at,
            1_790_000_120
        );
    }

    #[test]
    fn account_doc_encodes_absent_fields_as_missing() {
        let mut doc = AccountDoc::empty("A", 1_790_000_000);
        doc.session = Some("sealed".into());
        doc.app_secret_hash = Some("hash".into());
        doc.vault_expires_at = Some(1_790_086_400);
        doc.settings = Some(crate::store::AccountSettings {
            phone_wait_secs: 45,
            vault_ttl_secs: Some(86_400),
            always_use_vault: false,
        });
        let fields = account_fields(&doc);
        assert!(fields.get("vault").is_none());
        assert!(fields.get("fcmToken").is_none());
        assert_eq!(fields["session"], json!({ "stringValue": "sealed" }));
        assert_eq!(fields["appSecretHash"], json!({ "stringValue": "hash" }));
        let back = account_from_fields("A", fields.as_object().unwrap());
        assert!(back == doc);
    }

    #[tokio::test]
    async fn create_posts_with_document_id_and_bearer() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("{DOCS}/bridge_requests")))
            .and(query_param("documentId", "r1"))
            .and(header("authorization", "Bearer test-token"))
            .and(body_partial_json(
                json!({ "fields": { "status": { "stringValue": "pending" } } }),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;
        store(&server)
            .await
            .create_request(&request())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn firestore_get_404_is_none() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(json!({"error": {"status": "NOT_FOUND"}})),
            )
            .mount(&server)
            .await;
        let store = store(&server).await;
        assert!(store.get_request("r1").await.unwrap().is_none());
        assert!(store.get_account("A").await.unwrap().is_none());
        assert_eq!(store.username_for_fcm("h").await.unwrap(), None);
    }

    #[tokio::test]
    async fn complete_patches_with_update_time_precondition() {
        let server = MockServer::start().await;
        let fields = request_fields(&request());
        Mock::given(method("GET"))
            .and(path(format!("{DOCS}/bridge_requests/r1")))
            .respond_with(ResponseTemplate::new(200).set_body_json(doc_body(
                "bridge_requests/r1",
                fields,
                "2026-09-21T14:13:20.5Z",
            )))
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path(format!("{DOCS}/bridge_requests/r1")))
            .and(query_param(
                "currentDocument.updateTime",
                "2026-09-21T14:13:20.5Z",
            ))
            .and(body_partial_json(
                json!({ "fields": { "status": { "stringValue": "success" } } }),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;
        let completion = Completion {
            status: RequestStatus::Success,
            cookies_json: Some("[]".into()),
            error: None,
        };
        let doc = store(&server)
            .await
            .complete_request("r1", "tok", completion, 1_790_000_010)
            .await
            .unwrap();
        assert_eq!(doc.status, RequestStatus::Success);
        assert!(doc.response_token.is_none());
    }

    #[tokio::test]
    async fn firestore_complete_precondition_failure_is_already_handled() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(doc_body(
                "bridge_requests/r1",
                request_fields(&request()),
                "2026-09-21T14:13:20Z",
            )))
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .respond_with(ResponseTemplate::new(400).set_body_json(
                json!({ "error": { "code": 400, "status": "FAILED_PRECONDITION" } }),
            ))
            .mount(&server)
            .await;
        let completion = Completion {
            status: RequestStatus::Success,
            cookies_json: Some("[]".into()),
            error: None,
        };
        let result = store(&server)
            .await
            .complete_request("r1", "tok", completion, 1_790_000_010)
            .await;
        assert_eq!(result.err(), Some(CompleteError::AlreadyHandled));
    }

    #[tokio::test]
    async fn complete_bad_token_does_not_patch() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(doc_body(
                "bridge_requests/r1",
                request_fields(&request()),
                "2026-09-21T14:13:20Z",
            )))
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let completion = Completion {
            status: RequestStatus::Success,
            cookies_json: None,
            error: None,
        };
        let result = store(&server)
            .await
            .complete_request("r1", "nope", completion, 1_790_000_010)
            .await;
        assert_eq!(result.err(), Some(CompleteError::BadToken));
    }

    #[tokio::test]
    async fn delete_missing_is_ok_and_server_error_is_error() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path(format!("{DOCS}/bridge_requests/gone")))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path(format!("{DOCS}/bridge_requests/boom")))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let store = store(&server).await;
        store.delete_request("gone").await.unwrap();
        assert!(store.delete_request("boom").await.is_err());
    }

    #[tokio::test]
    async fn put_account_and_index_patch_whole_documents() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!("{DOCS}/accounts/A")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path(format!("{DOCS}/fcm_index/h")))
            .and(body_partial_json(
                json!({ "fields": { "username": { "stringValue": "A" } } }),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;
        let store = store(&server).await;
        store.put_account(&AccountDoc::empty("A", 1)).await.unwrap();
        store.put_fcm_index("h", "A").await.unwrap();
    }

    fn key() -> KeyDoc {
        KeyDoc {
            hash: "abc".into(),
            registration_number: "22BCE0001".into(),
            label: "laptop".into(),
            created_at: 1_790_000_000,
            last_used_at: Some(1_790_000_060),
        }
    }

    #[test]
    fn key_doc_encodes_and_decodes() {
        let fields = key_fields(&key());
        assert_eq!(
            fields["registrationNumber"],
            json!({ "stringValue": "22BCE0001" })
        );
        assert!(key_from_fields("abc", fields.as_object().unwrap()) == Some(key()));
    }

    #[tokio::test]
    async fn list_keys_runs_a_query_on_registration_number() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("{DOCS}:runQuery")))
            .and(body_partial_json(json!({ "structuredQuery": {
                "from": [{ "collectionId": "access_keys" }],
                "where": { "fieldFilter": {
                    "field": { "fieldPath": "registrationNumber" }, "op": "EQUAL",
                    "value": { "stringValue": "22BCE0001" } } } } })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                { "document": doc_body("access_keys/abc", key_fields(&key()), "2026-09-21T14:13:20Z") },
                { "readTime": "2026-09-21T14:13:20Z" }
            ])))
            .mount(&server)
            .await;
        let keys = store(&server).await.list_keys("22BCE0001").await.unwrap();
        assert!(keys == vec![key()]);
    }

    #[tokio::test]
    async fn delete_account_deletes_account_keys_and_requests() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("{DOCS}:runQuery")))
            .and(body_partial_json(json!({ "structuredQuery": { "from": [{ "collectionId": "access_keys" }] } })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                { "document": doc_body("access_keys/abc", key_fields(&key()), "2026-09-21T14:13:20Z") }
            ])))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("{DOCS}:runQuery")))
            .and(body_partial_json(json!({ "structuredQuery": { "from": [{ "collectionId": "bridge_requests" }] } })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                { "document": doc_body("bridge_requests/r1", request_fields(&request()), "2026-09-21T14:13:20Z") }
            ])))
            .mount(&server)
            .await;
        for doc in [
            "accounts/22BCE0001",
            "access_keys/abc",
            "bridge_requests/r1",
        ] {
            Mock::given(method("DELETE"))
                .and(path(format!("{DOCS}/{doc}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
                .expect(1)
                .mount(&server)
                .await;
        }
        store(&server)
            .await
            .delete_account("22BCE0001")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn health_lists_one_request() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("{DOCS}/bridge_requests")))
            .and(query_param("pageSize", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;
        store(&server).await.health().await.unwrap();
    }
}
