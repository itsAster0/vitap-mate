use std::time::Duration;

use scraper::{Html, Selector};

pub async fn find_captivative_portal() -> String {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let res = client
        .get("http://connectivitycheck.gstatic.com/generate_204")
        .send()
        .await;
    match res {
        Ok(k) => {
            if k.status().as_u16() == 204 {
                return "IE".into();
            }

            let text = k.text().await.unwrap_or("".into());
            let document = Html::parse_document(&text);
            let selector: Selector = Selector::parse("form[id=portal_url]").unwrap();

            for element in document.select(&selector) {
                if let Some(action) = element.value().attr("action") {
                    return action.to_string();
                }
            }
        }
        Err(_) => {
            return "NE".into();
        }
    }

    "".into()
}
