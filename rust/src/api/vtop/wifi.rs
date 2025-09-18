use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest;
use scraper::{Html, Selector};

pub async fn wifi_login_logout(i: i32, username: String, password: String) -> (bool, String) {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    if i == 0 {
        let res = client.get("http://172.18.10.10:1000/login?").send().await;
        match res {
            Ok(x) => {
                let res = x.text().await.unwrap_or("notext".to_string());

                let iip = parse_initil_page(res);

                if iip.0 {
                    let body = format!(
                        "magic={}&username={}&password={}",
                        iip.1, username, password
                    );
                    let res = client
                        .post("http://172.18.10.10:1000/?")
                        .body(body)
                        .send()
                        .await;
                    match res {
                        Ok(x) => {
                            let text = x.text().await.unwrap_or("notext".to_string());
                            let iip3 = get_em(text, "p");
                            if iip3.0 {
                                return (false, iip3.1);
                            } else {
                                let res = client
                                    .get("http://172.18.10.10:1000/keepalive?")
                                    .send()
                                    .await;
                                match res {
                                    Ok(x) => {
                                        let text = x.text().await.unwrap_or("notext".to_string());
                                        let iip2 = get_em(text, "h1");

                                        if iip.0 {
                                            return (true, iip2.1);
                                        } else {
                                            (false, "logined".to_string());
                                        }
                                    }
                                    Err(_e) => {
                                        return (false, "NE".to_string());
                                    }
                                }
                            }
                        }
                        Err(_e) => match client.get("https://1.1.1.1").send().await {
                            Ok(_) => {
                                return (
                                    false,
                                    "You are already connected to the internet.".to_string(),
                                );
                            }
                            Err(_) => return (false, "NE".to_string()),
                        },
                    }
                } else {
                    return (iip.0, "".to_string());
                }
            }
            Err(_e) => return (false, "NE".to_string()),
        }
    } else if i == 1 {
        let res = client.get("https://172.18.10.10:1000/logout?").send().await;
        match res {
            Ok(x) => {
                let text = x.text().await.unwrap_or("notext".to_string());

                let llp = get_em(text, "H3");
                if llp.0 {
                    return (true, llp.1);
                } else {
                    return (false, "NL".to_string());
                }
            }
            Err(_e) => {
                return (false, "NE".to_string());
            }
        }
    }

    (false, "".to_string())
}

fn parse_initil_page(res: String) -> (bool, String) {
    let document = Html::parse_document(&res);
    let selector = Selector::parse("input[type=hidden][name=magic]").unwrap();
    let page = document
        .select(&selector)
        .next()
        .and_then(|element| element.value().attr("value").map(|value| value.to_string()));
    match page {
        Some(k) => return (true, k),
        None => return (false, "".to_string()),
    }
}
fn get_em(text: String, sel: &str) -> (bool, String) {
    let document = Html::parse_document(&text);
    let selector = Selector::parse(sel).unwrap();
    let page = document.select(&selector).next();
    match page {
        Some(k) => {
            let o = k.text().collect::<Vec<_>>()[0];
            return (true, o.trim().to_string());
        }
        None => return (false, "".to_string()),
    }
}

pub async fn wifi_login_logout_hostel(
    i: i32,
    username: String,
    password: String,
) -> (bool, String) {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string();
    if i == 0 {
        let body = format!(
            "mode=191&username={}&password={}&a={}&producttype=0",
            username, password, timestamp
        );
        let res = client
            .post("https://172.18.8.14:8090/login.xml")
            .body(body)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await;
        match res {
            Ok(res) => {
                let text = res.text().await.unwrap_or_default();

                if text.to_lowercase().contains("you are signed in") {
                    return (true, "You have successfully logged in.".into());
                } else if text.to_lowercase().contains("limit reached") {
                    return (false, "Login failed. Limit Reached. ".into());
                }
            }
            Err(_e) => match client.get("https://1.1.1.1").send().await {
                Ok(_) => {
                    return (
                        false,
                        "You are already connected to the internet.".to_string(),
                    );
                }
                Err(_) => return (false, "NE".to_string()),
            },
        }
    } else {
        let body = format!(
            "mode=193&username={}&a={}&producttype=0",
            username, timestamp
        );
        let res = client
            .post("https://172.18.8.14:8090/logout.xml")
            .body(body)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await;
        match res {
            Ok(res) => {
                if res
                    .text()
                    .await
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains("signed out")
                {
                    return (true, "Signed out successfully.".into());
                }
            }
            Err(_e) => return (false, "NE".into()),
        }
    }

    (false, "".into())
}
