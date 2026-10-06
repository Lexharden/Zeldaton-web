//! The HTTP side: posting one message to a Discord webhook.

use std::time::Duration;

use serde_json::Value;

/// Only real Discord webhook URLs are accepted (so the server can never be pointed elsewhere).
pub fn valid_webhook_url(url: &str) -> bool {
    [
        "https://discord.com/api/webhooks/",
        "https://discordapp.com/api/webhooks/",
    ]
    .iter()
    .any(|prefix| url.starts_with(prefix) && url.len() > prefix.len())
}

/// A Discord role id: digits only (a "snowflake").
pub fn valid_role_id(id: &str) -> bool {
    (5..=25).contains(&id.len()) && id.chars().all(|c| c.is_ascii_digit())
}

pub struct DiscordClient {
    http: reqwest::Client,
    url: String,
}

impl DiscordClient {
    pub fn new(url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("http client");
        Self { http, url }
    }

    /// Posts `body`, retrying rate limits (429) and server errors (5xx) a few times. Errors never
    /// contain the webhook URL.
    pub async fn send(&self, body: &Value) -> Result<(), String> {
        let mut last = String::new();
        for attempt in 0..3u32 {
            match self.http.post(&self.url).json(body).send().await {
                Ok(res) if res.status().is_success() => return Ok(()),
                Ok(res) if res.status().as_u16() == 429 => {
                    let wait = res
                        .json::<Value>()
                        .await
                        .ok()
                        .and_then(|v| v["retry_after"].as_f64())
                        .unwrap_or(2.0)
                        .clamp(0.0, 10.0);
                    last = "Discord rate limit (429)".into();
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_secs_f64(wait)).await;
                    }
                }
                Ok(res) if res.status().is_server_error() => {
                    last = format!("Discord answered {}", res.status().as_u16());
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    }
                }
                Ok(res) => {
                    // 400/401/404...: a bad or deleted webhook. Retrying cannot help.
                    return Err(format!(
                        "Discord rejected the message ({}): check the webhook URL",
                        res.status().as_u16()
                    ));
                }
                Err(e) => {
                    last = format!("could not reach Discord: {}", e.without_url());
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    }
                }
            }
        }
        Err(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_discord_webhooks_and_numeric_roles_are_accepted() {
        assert!(valid_webhook_url("https://discord.com/api/webhooks/1/abc"));
        assert!(valid_webhook_url(
            "https://discordapp.com/api/webhooks/1/abc"
        ));
        for bad in [
            "",
            "https://discord.com/api/webhooks/",
            "http://discord.com/api/webhooks/1/abc",
            "https://evil.example/api/webhooks/1/abc",
            "https://discord.com.evil.example/api/webhooks/1/abc",
        ] {
            assert!(!valid_webhook_url(bad), "{bad}");
        }
        assert!(valid_role_id("123456789012345678"));
        for bad in [
            "",
            "1234",
            "12a456",
            "<@&123456>",
            "123456789012345678901234567",
        ] {
            assert!(!valid_role_id(bad), "{bad}");
        }
    }
}
