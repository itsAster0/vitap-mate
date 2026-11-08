//! Saves real VTOP login CAPTCHAs with today's solver answers, for testing a
//! new decoder: `cargo run -p vtop-core --example captcha_samples -- [count]`.
//!
//! Each sample opens a fresh session (GET /vtop/open/page, then the prelogin
//! POST) and never signs in. Writes `rust/vtop-proto/tests/captchas/<n>.jpg`
//! and `answers.json`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use vtop_core::parser::page;

const BASE: &str = "https://vtop.vitap.ac.in";
const CA_PEM: &str = include_str!("../assets/sectigo_dv_r36.pem");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count: usize = std::env::args()
        .nth(1)
        .map(|n| n.parse())
        .transpose()?
        .unwrap_or(50);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vtop-proto/tests/captchas");
    std::fs::create_dir_all(&dir)?;
    let mut answers = BTreeMap::new();
    let mut n = 0;
    while n < count {
        tokio::time::sleep(Duration::from_millis(1200)).await;
        match sample().await {
            Ok((bytes, answer)) => {
                n += 1;
                let name = format!("{n}.jpg");
                std::fs::write(dir.join(&name), bytes)?;
                println!("{name}: {answer}");
                answers.insert(name, answer);
            }
            Err(error) => eprintln!("sample failed: {error}"),
        }
    }
    std::fs::write(
        dir.join("answers.json"),
        serde_json::to_string_pretty(&answers)? + "\n",
    )?;
    Ok(())
}

/// One fresh session: the raw CAPTCHA image and the solver's answer.
async fn sample() -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let http = reqwest::Client::builder()
        .cookie_store(true)
        .add_root_certificate(reqwest::Certificate::from_pem(CA_PEM.as_bytes())?)
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:157.0) Gecko/20100101 Firefox/157.0")
        .timeout(Duration::from_secs(20))
        .build()?;
    let landing = http
        .get(format!("{BASE}/vtop/open/page"))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let csrf = page::extract_csrf_token(&landing).ok_or("landing page had no CSRF token")?;
    let prelogin = http
        .post(format!("{BASE}/vtop/prelogin/setup"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("_csrf={csrf}&flag=VTOP"))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let data_url = page::extract_captcha_data(&prelogin).ok_or("prelogin page had no CAPTCHA")?;
    let answer = vtop_core::solve_captcha(&data_url)?;
    let (_, encoded) = data_url.split_once(',').ok_or("bad data URL")?;
    Ok((STANDARD.decode(encoded)?, answer))
}
