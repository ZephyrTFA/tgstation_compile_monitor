use serde::Serialize;

use crate::conf::TargetInfo;

#[derive(Serialize)]
struct WebhookPostData {
    content: String,
    username: Option<String>,
}

pub async fn post_to_webhook(message: &str, cfg: &TargetInfo) {
    let data: WebhookPostData = WebhookPostData {
        content: message.to_string(),
        username: cfg.name_override().clone(),
    };

    let client = reqwest::Client::new();
    let response = client
        .post(cfg.webhook_url())
        .header("user-agent", "Yaaw")
        .header("content-type", "application/json")
        .query(&[("wait", true)])
        .json(&data)
        .send()
        .await
        .expect("failed to post message");
    if !response.status().is_success() {
        println!(
            "Failed to post message to webhook: {}",
            response.text().await.unwrap()
        );
    }
}
