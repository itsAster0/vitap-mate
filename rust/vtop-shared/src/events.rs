//! Events the edge sends to an app install. Every event has a sequence
//! number in that install's log; the same event may arrive over the
//! WebSocket, FCM and a catch-up read, and the app applies each number once.

use serde::{Deserialize, Serialize};

/// FCM data messages are capped at 4 KB; an event stays under it.
pub const MAX_EVENT_BYTES: usize = 3584;
/// How long the log keeps an event for catch-up.
pub const EVENT_TTL_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventBody {
    /// A forwarded mail, sealed to the install's mail key.
    MailOtp { mailbox: String, sealed: String },
    /// An app or agent wants a VTOP session: the phone answers with its
    /// cookies at `callback_url`. Fields are strings, as FCM data is, so
    /// older app builds read the push unchanged.
    #[serde(rename = "vtop_cookie_request", rename_all = "camelCase")]
    CookieRequest {
        request_id: String,
        response_token: String,
        callback_url: String,
        /// `"1"` when saved credentials should come with the answer.
        want_credentials: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub seq: u64,
    /// Unix seconds.
    pub at: u64,
    #[serde(flatten)]
    pub body: EventBody,
}

/// What the edge sends over the socket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerFrame {
    Event {
        event: Event,
    },
    /// Sent once after connecting: events after the client's cursor follow.
    Hello {
        latest: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_json_is_flat_and_tagged() {
        let event = Event {
            seq: 3,
            at: 10,
            body: EventBody::MailOtp {
                mailbox: "abcdefghij".into(),
                sealed: "AQ".into(),
            },
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"seq": 3, "at": 10, "kind": "mail_otp", "mailbox": "abcdefghij", "sealed": "AQ"})
        );
        assert_eq!(serde_json::from_value::<Event>(json).unwrap(), event);
    }

    #[test]
    fn cookie_requests_read_like_the_old_push() {
        let body = EventBody::CookieRequest {
            request_id: "r".into(),
            response_token: "t".into(),
            callback_url: "https://edge/v1/session/answer?account=A".into(),
            want_credentials: "0".into(),
        };
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            serde_json::json!({"kind": "vtop_cookie_request", "requestId": "r", "responseToken": "t",
                "callbackUrl": "https://edge/v1/session/answer?account=A", "wantCredentials": "0"})
        );
    }
}
