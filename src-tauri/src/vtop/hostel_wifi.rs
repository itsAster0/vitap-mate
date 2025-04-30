use tauri_plugin_http::reqwest;
use std::time::{SystemTime, UNIX_EPOCH};

// Changed parameter name from "action" to "i" to match command expectation
pub async fn hostel_wifi(i: i32, username: String, password: String) -> (bool, String) {
    // Create a reqwest client that ignores SSL certificate validation
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    
    // Get current timestamp for the 'a' parameter
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string();

    // Define common headers
    let headers = [
        ("Accept", "*/*"),
        ("Accept-Language", "en-GB,en-US;q=0.9,en;q=0.8"),
        ("Connection", "keep-alive"),
        ("Content-Type", "application/x-www-form-urlencoded"),
        ("Origin", "https://172.18.8.8:8090"),
        ("Referer", "https://172.18.8.8:8090/httpclient.html"),
        ("Sec-Fetch-Dest", "empty"),
        ("Sec-Fetch-Mode", "cors"),
        ("Sec-Fetch-Site", "same-origin"),
        ("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36"),
        ("sec-ch-ua", "\"Google Chrome\";v=\"135\", \"Not-A.Brand\";v=\"8\", \"Chromium\";v=\"135\""),
        ("sec-ch-ua-mobile", "?0"),
        ("sec-ch-ua-platform", "\"Linux\"")
    ];

    if i == 0 {
        // Login action
        let url = "https://172.18.8.8:8090/login.xml";
        let body = format!(
            "mode=191&username={}&password={}&a={}&producttype=0",
            username, password, timestamp
        );

        // Send login request
        let res = client
            .post(url)
            .headers(reqwest::header::HeaderMap::from_iter(
                headers.iter().map(|(k, v)| {
                    (
                        reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                        reqwest::header::HeaderValue::from_str(v).unwrap(),
                    )
                })
            ))
            .body(body)
            .send()
            .await;

        match res {
            Ok(response) => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                
                // Check if the response contains successful login indication
                // Note: You may need to adjust this based on the actual response format
                if status.is_success() && !text.contains("ERROR") {
                    return (true, "Successfully logged in".to_string());
                } else {
                    return (false, format!("Login failed: {}", text));
                }
            }
            Err(e) => {
                return (false, format!("Network error: {}", e));
            }
        }
    } else if i == 1 {
        // Logout action
        let url = "https://172.18.8.8:8090/logout.xml";
        let body = format!(
            "mode=193&username={}&a={}&producttype=0",
            username, timestamp
        );

        // Send logout request
        let res = client
            .post(url)
            .headers(reqwest::header::HeaderMap::from_iter(
                headers.iter().map(|(k, v)| {
                    (
                        reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                        reqwest::header::HeaderValue::from_str(v).unwrap(),
                    )
                })
            ))
            .body(body)
            .send()
            .await;

        match res {
            Ok(response) => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                
                // Check if the response contains successful logout indication
                if status.is_success() && !text.contains("ERROR") {
                    return (true, "Successfully logged out".to_string());
                } else {
                    return (false, format!("Logout failed: {}", text));
                }
            }
            Err(e) => {
                return (false, format!("Network error: {}", e));
            }
        }
    }

    // If action is neither login nor logout
    (false, "Invalid action".to_string())
}
