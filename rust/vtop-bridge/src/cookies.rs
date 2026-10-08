//! Cookie-Editor cookies, the format the extension installs, and the
//! conversion to and from a `Cookie` header.

use serde::{Deserialize, Serialize};

pub const VTOP_DOMAIN: &str = "vtop.vitap.ac.in";

/// Field names and omissions match the Go bridge. No `Debug`: it holds a
/// cookie value.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CookieEditorCookie {
    pub domain: String,
    pub host_only: bool,
    pub http_only: bool,
    pub name: String,
    pub path: String,
    pub same_site: String,
    pub secure: bool,
    pub session: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_id: Option<String>,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
}

/// Non-empty, and every cookie has a name, a value and a domain.
pub fn validate(cookies: &[CookieEditorCookie]) -> Result<(), &'static str> {
    if cookies.is_empty() {
        return Err("empty cookie list");
    }
    for cookie in cookies {
        if cookie.name.trim().is_empty() {
            return Err("cookie missing name");
        }
        if cookie.value.is_empty() {
            return Err("cookie missing value");
        }
        if cookie.domain.trim().is_empty() {
            return Err("cookie missing domain");
        }
    }
    Ok(())
}

/// `name=value; name=value`.
pub fn to_header(cookies: &[CookieEditorCookie]) -> String {
    cookies
        .iter()
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ")
}

/// The same cookies the app builds in `cookieEditorCookiesFromHeader`.
pub fn from_header(header: &str) -> Vec<CookieEditorCookie> {
    header
        .split(';')
        .enumerate()
        .filter_map(|(index, part)| {
            let (name, value) = part.trim().split_once('=')?;
            let (name, value) = (name.trim(), value.trim());
            if name.is_empty() || value.is_empty() {
                return None;
            }
            Some(CookieEditorCookie {
                domain: VTOP_DOMAIN.into(),
                host_only: true,
                http_only: false,
                name: name.into(),
                path: "/".into(),
                same_site: "unspecified".into(),
                secure: true,
                session: true,
                store_id: Some("0".into()),
                value: value.into(),
                expiration_date: None,
                id: Some(index as i64 + 1),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn from_header_matches_app_cookie_editor_shape() {
        let cookies = from_header("JSESSIONID=abc; ; bad; SERVERID=x=y");
        let json = serde_json::to_value(&cookies).unwrap();
        assert_eq!(
            json,
            json!([
                { "domain": "vtop.vitap.ac.in", "hostOnly": true, "httpOnly": false,
                  "name": "JSESSIONID", "path": "/", "sameSite": "unspecified",
                  "secure": true, "session": true, "storeId": "0", "value": "abc", "id": 1 },
                { "domain": "vtop.vitap.ac.in", "hostOnly": true, "httpOnly": false,
                  "name": "SERVERID", "path": "/", "sameSite": "unspecified",
                  "secure": true, "session": true, "storeId": "0", "value": "x=y", "id": 4 }
            ])
        );
    }

    #[test]
    fn header_round_trip() {
        assert_eq!(to_header(&from_header("A=1; B=2")), "A=1; B=2");
    }

    #[test]
    fn missing_fields_deserialize_to_defaults() {
        let cookie: CookieEditorCookie =
            serde_json::from_value(json!({ "domain": "d", "name": "n", "value": "v" })).unwrap();
        assert_eq!(cookie.path, "");
        assert!(cookie.id.is_none());
        let back = serde_json::to_value(&cookie).unwrap();
        assert!(back.get("storeId").is_none());
        assert!(back.get("expirationDate").is_none());
    }

    #[test]
    fn validate_requires_name_value_domain() {
        let good = CookieEditorCookie {
            domain: "d".into(),
            name: "n".into(),
            value: "v".into(),
            ..Default::default()
        };
        assert!(validate(std::slice::from_ref(&good)).is_ok());
        assert!(validate(&[]).is_err());
        assert!(validate(&[CookieEditorCookie {
            name: " ".into(),
            ..good.clone()
        }])
        .is_err());
        assert!(validate(&[CookieEditorCookie {
            value: String::new(),
            ..good.clone()
        }])
        .is_err());
        assert!(validate(&[CookieEditorCookie {
            domain: " ".into(),
            ..good
        }])
        .is_err());
    }
}
