//! VTOP's sign-in OTP in a Gmail API message (`users.messages.get` with
//! `format=full`), without I/O. The app (Kotlin, over FFI) and vtop-core
//! fetch the message and hand it here, so both read it the same way: a mail
//! from [`SENDER`] received after the OTP was issued, and the first
//! standalone 6-digit number in its snippet or body parts (HTML parts as
//! their text, without tags, styles or scripts).

use serde::Deserialize;

use crate::captcha::base64_url;

/// The address VTOP's OTP mails come from.
pub const SENDER: &str = "noreply.sdc@vitap.ac.in";

/// The fields of a Gmail API message the OTP is read from. No `Debug`: it
/// holds the mail.
#[derive(Deserialize, Default)]
pub struct Message {
    pub snippet: Option<String>,
    /// Milliseconds since the unix epoch, as a decimal string.
    #[serde(rename = "internalDate")]
    pub internal_date: Option<String>,
    pub payload: Option<Part>,
}

/// One MIME part; the payload is the root one.
#[derive(Deserialize, Default)]
pub struct Part {
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    pub headers: Option<Vec<Header>>,
    pub body: Option<Body>,
    pub parts: Option<Vec<Part>>,
}

#[derive(Deserialize, Default)]
pub struct Header {
    pub name: Option<String>,
    pub value: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct Body {
    /// URL-safe base64.
    pub data: Option<String>,
}

/// The OTP in `message` when VTOP sent it at or after `since` (unix
/// seconds); `None` for an older mail, another sender or no code.
pub fn otp(message: &Message, since: u64) -> Option<String> {
    let received_ms = message
        .internal_date
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    if received_ms < since.saturating_mul(1000) || !from_vtop(message) {
        return None;
    }
    first_six_digit_code(&message_text(message))
}

/// A `From` header (any case) whose address is exactly [`SENDER`] (any
/// case).
fn from_vtop(message: &Message) -> bool {
    message
        .payload
        .as_ref()
        .and_then(|payload| payload.headers.as_deref())
        .unwrap_or_default()
        .iter()
        .any(|header| {
            header
                .name
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case("from"))
                && header
                    .value
                    .as_deref()
                    .is_some_and(|value| address(value).eq_ignore_ascii_case(SENDER))
        })
}

/// The address in a `From` value: inside the last `<...>` when there is
/// one, else the whole value, trimmed.
fn address(from: &str) -> &str {
    from.rsplit_once('<')
        .and_then(|(_, rest)| rest.split_once('>'))
        .map_or(from, |(address, _)| address)
        .trim()
}

/// Snippet plus every decoded body part, quoted-printable decoded; HTML
/// parts as their text.
fn message_text(message: &Message) -> String {
    fn append(part: &Part, out: &mut String) {
        let data = part.body.as_ref().and_then(|body| body.data.as_deref());
        if let Some(data) = data.filter(|data| !data.is_empty())
            && let Some(bytes) = base64_url(data.trim_end_matches('=').as_bytes())
        {
            let text = decode_quoted_printable(&String::from_utf8_lossy(&bytes));
            let html = part
                .mime_type
                .as_deref()
                .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("text/html"));
            out.push_str(&if html { html_text(&text) } else { text });
            out.push('\n');
        }
        for child in part.parts.as_deref().unwrap_or_default() {
            append(child, out);
        }
    }
    let mut text = String::new();
    if let Some(snippet) = &message.snippet {
        text.push_str(&decode_quoted_printable(snippet));
        text.push('\n');
    }
    if let Some(payload) = &message.payload {
        append(payload, &mut text);
    }
    text
}

/// The text of an HTML document: `<style>` and `<script>` contents,
/// comments and tags dropped, each tag leaving a space so text in separate
/// cells does not run together.
fn html_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((text, tag)) = rest.split_once('<') {
        out.push_str(text);
        out.push(' ');
        let name: String = tag
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_lowercase();
        let after = if let Some(comment) = tag.strip_prefix("!--") {
            comment.split_once("-->").map(|(_, after)| after)
        } else if name == "style" || name == "script" {
            after_closing(tag, &name)
        } else {
            tag.split_once('>').map(|(_, after)| after)
        };
        let Some(after) = after else {
            return out;
        };
        rest = after;
    }
    out.push_str(rest);
    out
}

/// What follows the first `</name ...>` (any case) in `html`.
fn after_closing<'a>(html: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = html;
    loop {
        let (_, after) = rest.split_once("</")?;
        if after
            .get(..name.len())
            .is_some_and(|tag| tag.eq_ignore_ascii_case(name))
        {
            return after.split_once('>').map(|(_, after)| after);
        }
        rest = after;
    }
}

/// Joins soft line breaks and turns each `=XX` (two hex digits) into its
/// byte; any other `=` stays as it is.
fn decode_quoted_printable(input: &str) -> String {
    let joined = input.replace("=\r\n", "").replace("=\n", "");
    let mut rest = joined.as_bytes();
    let mut out = Vec::with_capacity(rest.len());
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'='
            && let Some((&[high, low], after)) = tail.split_first_chunk::<2>()
            && let (Some(high), Some(low)) = (hex(high), hex(low))
        {
            out.push(high << 4 | low);
            rest = after;
            continue;
        }
        out.push(byte);
        rest = tail;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(byte: u8) -> Option<u8> {
    char::from(byte)
        .to_digit(16)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// The first run of exactly six ASCII digits.
fn first_six_digit_code(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_digit())
        .find(|run| run.len() == 6)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// URL-safe base64 without padding, for building bodies.
    fn encode(text: &str) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = String::new();
        for chunk in text.as_bytes().chunks(3) {
            let mut buf = [0_u8; 3];
            buf[..chunk.len()].copy_from_slice(chunk);
            let n = u32::from(buf[0]) << 16 | u32::from(buf[1]) << 8 | u32::from(buf[2]);
            for i in 0..=chunk.len() {
                out.push(char::from(ALPHABET[(n >> (18 - 6 * i) & 63) as usize]));
            }
        }
        out
    }

    fn mail(from: &str, snippet: &str, parts: serde_json::Value) -> Message {
        serde_json::from_value(serde_json::json!({
            "id": "m1",
            "internalDate": "1763700000000",
            "snippet": snippet,
            "payload": {
                "mimeType": "multipart/alternative",
                "headers": [
                    {"name": "Subject", "value": "VTOP Login OTP"},
                    {"name": "From", "value": from}
                ],
                "body": {"size": 0},
                "parts": parts
            }
        }))
        .unwrap()
    }

    const VTOP: &str = "VTOP <noreply.sdc@vitap.ac.in>";
    const SINCE: u64 = 1_763_699_990;

    #[test]
    fn plain_text_mail() {
        let message = mail(
            VTOP,
            "",
            serde_json::json!([{
                "mimeType": "text/plain",
                "body": {"data": encode("Dear student, your OTP is 401295. Valid 5 min.\r\n")}
            }]),
        );
        assert_eq!(otp(&message, SINCE).as_deref(), Some("401295"));
    }

    #[test]
    fn html_only_mail() {
        let html = "<html><body><p style=\"color:#123456\">Your OTP</p>\
                    <b>738201</b><p>ref 12345678</p></body></html>";
        let message = mail(
            "noreply.sdc@vitap.ac.in",
            "",
            serde_json::json!([{
                "mimeType": "text/html",
                "body": {"data": encode(html)}
            }]),
        );
        assert_eq!(otp(&message, SINCE).as_deref(), Some("738201"));
    }

    #[test]
    fn quoted_printable_mail() {
        let body = "<td style=3D\"font-size:20px\">OTP: 11=\r\n2233</td>";
        let message = mail(
            VTOP,
            "",
            serde_json::json!([{"mimeType": "text/html", "body": {"data": encode(body)}}]),
        );
        assert_eq!(otp(&message, SINCE).as_deref(), Some("112233"));
        assert_eq!(decode_quoted_printable("a=3Db=\nc=3d"), "a=bc=");
        // Only two hex digits make an escape; `=+1` is not one.
        assert_eq!(decode_quoted_printable("x=+1=4"), "x=+1=4");
    }

    #[test]
    fn snippet_and_nested_parts_are_read() {
        let message = mail(
            "VTOP <NOREPLY.SDC@VITAP.AC.IN>",
            "Security OTP",
            serde_json::json!([{
                "mimeType": "multipart/related",
                "parts": [{"body": {"data": format!("{}==", encode("code 654321"))}}]
            }]),
        );
        assert_eq!(otp(&message, SINCE).as_deref(), Some("654321"));
    }

    #[test]
    fn wrong_sender() {
        let message = mail(
            "someone@example.com",
            "Your OTP is 401295",
            serde_json::json!([]),
        );
        assert_eq!(otp(&message, SINCE), None);
        let no_headers: Message = serde_json::from_value(serde_json::json!({
            "internalDate": "1763700000000",
            "snippet": "Your OTP is 401295"
        }))
        .unwrap();
        assert_eq!(otp(&no_headers, SINCE), None);
    }

    #[test]
    fn lookalike_senders_are_rejected() {
        for from in [
            "fake-noreply.sdc@vitap.ac.in",
            "VTOP <fake-noreply.sdc@vitap.ac.in>",
            "noreply.sdc@vitap.ac.in.example.com",
            "\"noreply.sdc@vitap.ac.in\" <someone@example.com>",
        ] {
            let message = mail(from, "Your OTP is 401295", serde_json::json!([]));
            assert_eq!(otp(&message, SINCE), None, "{from}");
        }
    }

    #[test]
    fn display_name_and_angle_brackets_are_accepted() {
        for from in [
            "VIT-AP SDC <noreply.sdc@vitap.ac.in>",
            "\"VTOP, VIT-AP\" < Noreply.SDC@vitap.ac.in >",
            "  noreply.sdc@vitap.ac.in  ",
        ] {
            let message = mail(from, "Your OTP is 401295", serde_json::json!([]));
            assert_eq!(otp(&message, SINCE).as_deref(), Some("401295"), "{from}");
        }
    }

    #[test]
    fn html_styles_and_scripts_are_not_read() {
        let html = "<html><head><style>.otp { color: #654321; } td{background:#112233}</style>\
                    <script type=\"text/javascript\">var n = 999999;</script></head>\
                    <body bgcolor=\"#223344\"><!-- id 555555 --><table><tr><td>123</td><td>456</td></tr></table>\
                    <p style=\"color:#123456\">Your OTP is <b>482913</b></p></body></html>";
        let message = mail(
            VTOP,
            "",
            serde_json::json!([{"mimeType": "text/html", "body": {"data": encode(html)}}]),
        );
        assert_eq!(otp(&message, SINCE).as_deref(), Some("482913"));
        assert_eq!(html_text("a<STYLE>x 111111</style >b<br/>c"), "a b c");
        // An unclosed tag ends the text.
        assert_eq!(html_text("ok <p class=\"x"), "ok  ");
    }

    #[test]
    fn mail_with_no_code() {
        let message = mail(
            VTOP,
            "Your password was changed",
            serde_json::json!([{"body": {"data": encode("ref 12345 and 1234567")}}]),
        );
        assert_eq!(otp(&message, SINCE), None);
    }

    #[test]
    fn older_mail_is_skipped() {
        let message = mail(VTOP, "Your OTP is 401295", serde_json::json!([]));
        assert_eq!(otp(&message, SINCE + 11).as_deref(), None);
        assert_eq!(otp(&message, SINCE + 10).as_deref(), Some("401295"));
    }

    #[test]
    fn bad_base64_part_is_skipped() {
        let message = mail(
            VTOP,
            "",
            serde_json::json!([
                {"body": {"data": "not base64!"}},
                {"body": {"data": encode("OTP 990011")}}
            ]),
        );
        assert_eq!(otp(&message, SINCE).as_deref(), Some("990011"));
    }

    #[test]
    fn six_digit_runs() {
        assert_eq!(
            first_six_digit_code("ref 12345678 then 654321").as_deref(),
            Some("654321")
        );
        assert_eq!(first_six_digit_code("no code 12345"), None);
    }
}
