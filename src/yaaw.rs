use std::{fs, time::SystemTime};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct YelledAboutAndWhen {
    webhook: String,
    server: String,
    when: SystemTime,
}

impl YelledAboutAndWhen {
    pub fn new(webhook: &str, server: &str, when: SystemTime) -> Self {
        Self {
            webhook: webhook.into(),
            server: server.into(),
            when,
        }
    }

    pub fn webhook(&self) -> &str {
        &self.webhook
    }

    pub fn server(&self) -> &str {
        &self.server
    }

    pub fn when(&self) -> SystemTime {
        self.when
    }
}

pub fn save_yaaw(yaaw: &Vec<YelledAboutAndWhen>) {
    let webhook_url = yaaw.first().map(|x| x.webhook());
    if webhook_url.is_none() {
        println!("no data to save");
        return;
    }

    let webhook_id = webhook_url_to_id(webhook_url.unwrap());
    // ensure the directory exists
    fs::create_dir_all("./yaaw").expect("failed to create yaaw directory");
    serde_json::to_writer(
        &fs::File::create(format!("./yaaw/{}.json", webhook_id))
            .expect("failed to create yaaw file."),
        yaaw,
    )
    .expect("failed to write yaaw file.");
}

pub fn load_yaaw(webhook_url: &str) -> Vec<YelledAboutAndWhen> {
    let webhook_id = webhook_url_to_id(webhook_url);
    println!("loading yaaw for {}", webhook_id);
    let file = std::fs::read(format!("./yaaw/{}.json", webhook_id));
    if let Ok(file) = file {
        serde_json::from_slice(&file).unwrap()
    } else {
        Vec::new()
    }
}

fn webhook_url_to_id(webhook_url: &str) -> String {
    let webhook_parts: Vec<&str> = webhook_url.split('/').collect();
    webhook_parts
        .iter()
        .rev()
        .nth(1)
        .expect("failed to extract webhook id")
        .to_string()
}
