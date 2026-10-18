//! What a forwarded mail means for the phone. Only two kinds are relayed:
//! VTOP's OTP mail, and Gmail's forwarding confirmation (needed once during
//! setup). Everything else is dropped. The mail is parsed in memory and
//! nothing about its content is logged.

use mail_parser::MessageParser;
use serde::Serialize;

pub const VTOP_SENDER: &str = "noreply.sdc@vitap.ac.in";
pub const GMAIL_FORWARDING_SENDER: &str = "forwarding-noreply@google.com";
const CONFIRM_LINK: &str = "https://mail-settings.google.com/mail/vf-";

/// The plaintext sealed for the phone; the app reads it as JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Payload {
    Otp {
        code: String,
        #[serde(rename = "sentAt")]
        sent_at: u64,
    },
    GmailConfirm {
        code: Option<String>,
        link: Option<String>,
        requester: Option<String>,
        #[serde(rename = "sentAt")]
        sent_at: u64,
    },
}

impl Payload {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Otp { .. } => "otp",
            Self::GmailConfirm { .. } => "gmail_confirm",
        }
    }

    pub fn sent_at(&self) -> u64 {
        match self {
            Self::Otp { sent_at, .. } | Self::GmailConfirm { sent_at, .. } => *sent_at,
        }
    }
}

/// The first run of exactly six digits, as vtop-core's Gmail reader.
pub fn first_six_digit_code(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_digit())
        .find(|run| run.len() == 6)
        .map(str::to_string)
}

fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let lower = html.to_ascii_lowercase();
    let mut i = 0;
    while i < html.len() {
        let rest = &lower[i..];
        if let Some(skip) = ["<style", "<script"]
            .iter()
            .find(|tag| rest.starts_with(**tag))
            .map(|tag| format!("</{}", &tag[1..]))
        {
            i = match rest.find(&skip) {
                Some(end) => i + end + skip.len(),
                None => html.len(),
            };
            out.push(' ');
            continue;
        }
        if rest.starts_with('<') {
            i = rest.find('>').map_or(html.len(), |end| i + end + 1);
            out.push(' ');
            continue;
        }
        let ch = html[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out.replace("&nbsp;", " ").replace("&amp;", "&")
}

/// Digits following `label` (case-insensitive), after optional spaces.
fn digits_after(text: &str, label: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let start = lower.find(label)? + label.len();
    let digits: String = text[start..]
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    (!digits.is_empty()).then_some(digits)
}

fn confirm_link(text: &str) -> Option<String> {
    let start = text.find(CONFIRM_LINK)?;
    let link: String = text[start..]
        .chars()
        .take_while(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | '<' | '>'))
        .collect();
    (link.len() > CONFIRM_LINK.len()).then_some(link)
}

fn requester(subject: &str) -> Option<String> {
    let lower = subject.to_ascii_lowercase();
    let label = "receive mail from ";
    let start = lower.find(label)? + label.len();
    let word = subject[start..].split_whitespace().next()?;
    word.contains('@').then(|| word.to_string())
}

/// The payload for a raw RFC 5322 mail, or `None` to drop it. `now` is used
/// when the mail has no usable `Date:`.
pub fn payload_for(raw: &[u8], now: u64) -> Option<Payload> {
    let message = MessageParser::default().parse(raw)?;
    let from = message
        .from()
        .and_then(|from| from.first())
        .and_then(|addr| addr.address())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let subject = message.subject().unwrap_or_default().to_string();
    let text = [
        subject.clone(),
        message
            .body_text(0)
            .map(|t| t.into_owned())
            .unwrap_or_default(),
        message
            .body_html(0)
            .map(|h| html_to_text(&h))
            .unwrap_or_default(),
    ]
    .join("\n");
    let sent_at = message
        .date()
        .map(|date| date.to_timestamp())
        .filter(|ts| *ts > 0)
        .map_or(now, |ts| ts as u64);

    match from.as_str() {
        VTOP_SENDER => first_six_digit_code(&text).map(|code| Payload::Otp { code, sent_at }),
        GMAIL_FORWARDING_SENDER => {
            let code =
                digits_after(&text, "confirmation code:").or_else(|| digits_after(&text, "(#"));
            let link = confirm_link(&text);
            if code.is_none() && link.is_none() {
                return None;
            }
            Some(Payload::GmailConfirm {
                code,
                link,
                requester: requester(&subject),
                sent_at,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(from: &str, subject: &str, body: &str, content_type: &str) -> Vec<u8> {
        [
            format!("From: {from}"),
            "To: someone@gmail.com".into(),
            format!("Subject: {subject}"),
            "Date: Fri, 17 Oct 2026 10:00:00 +0000".into(),
            "MIME-Version: 1.0".into(),
            format!("Content-Type: {content_type}; charset=UTF-8"),
            "Content-Transfer-Encoding: quoted-printable".into(),
            String::new(),
            body.into(),
        ]
        .join("\r\n")
        .into_bytes()
    }

    #[test]
    fn six_digit_rule_matches_vtop_core() {
        assert_eq!(
            first_six_digit_code("Your OTP is 401295. Valid 5 min").as_deref(),
            Some("401295")
        );
        assert_eq!(
            first_six_digit_code("ref 12345678 then 654321").as_deref(),
            Some("654321")
        );
        assert_eq!(first_six_digit_code("no code 12345"), None);
    }

    #[test]
    fn vtop_otp_in_quoted_printable_html() {
        let mail = raw(
            "VTOP <noreply.sdc@vitap.ac.in>",
            "VTOP Login OTP",
            "<p style=3D\"x\">Ref 2026 Your OTP is <b>112233</b></p>",
            "text/html",
        );
        assert_eq!(
            payload_for(&mail, 1),
            Some(Payload::Otp {
                code: "112233".into(),
                sent_at: 1792231200
            })
        );
    }

    #[test]
    fn styles_and_scripts_are_not_read() {
        let mail = raw(
            "noreply.sdc@vitap.ac.in",
            "VTOP Login OTP",
            "<style>.a{width:100000px}</style><script>var x=999999</script>OTP 445566",
            "text/html",
        );
        assert!(
            matches!(payload_for(&mail, 1), Some(Payload::Otp { code, .. }) if code == "445566")
        );
    }

    #[test]
    fn gmail_forwarding_confirmation() {
        let body = [
            "student@gmail.com has requested to automatically forward mail to your email",
            "Confirmation code: 987654321",
            "To allow, click the link below:",
            "https://mail-settings.google.com/mail/vf-%5BANGjdJ8%5D-abc123",
        ]
        .join("\r\n");
        let mail = raw(
            "Gmail Team <forwarding-noreply@google.com>",
            "(#987654321) Gmail Forwarding Confirmation - Receive Mail from student@gmail.com",
            &body,
            "text/plain",
        );
        assert_eq!(
            payload_for(&mail, 1),
            Some(Payload::GmailConfirm {
                code: Some("987654321".into()),
                link: Some("https://mail-settings.google.com/mail/vf-%5BANGjdJ8%5D-abc123".into()),
                requester: Some("student@gmail.com".into()),
                sent_at: 1792231200,
            })
        );
    }

    #[test]
    fn other_senders_and_codeless_mail_are_dropped() {
        assert_eq!(
            payload_for(
                &raw("spam@example.com", "OTP 123456", "123456", "text/plain"),
                1
            ),
            None
        );
        assert_eq!(
            payload_for(
                &raw(
                    "noreply.sdc@vitap.ac.in",
                    "Hello",
                    "no code here",
                    "text/plain"
                ),
                1
            ),
            None
        );
    }

    #[test]
    fn payload_json_matches_what_the_app_reads() {
        let otp = Payload::Otp {
            code: "112233".into(),
            sent_at: 5,
        };
        assert_eq!(
            serde_json::to_value(&otp).unwrap(),
            serde_json::json!({"kind": "otp", "code": "112233", "sentAt": 5})
        );
    }
}
