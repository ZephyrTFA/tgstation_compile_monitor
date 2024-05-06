use std::{fs, time::{SystemTime, UNIX_EPOCH}};

use serde::{Deserialize, Serialize};

mod fetch_compile_data;

const QUERY_INTEVAL_MINUTES: u64 = 60;
const ERROR_REVISION_DATE_UNCHANGED_FOR_HOURS: u64 = 24;

fn save_yaaw(yaaw: &Vec<YelledAboutAndWhen>) {
    std::fs::write(
        "yelled_about_and_when.json",
        serde_json::to_string(yaaw).unwrap(),
    ).expect("Failed to save yelled about and when");
}

fn load_yaaw() -> Vec<YelledAboutAndWhen> {
    let file = std::fs::read("yelled_about_and_when.json");
    if let Ok(file) = file {
        serde_json::from_slice(&file).unwrap()
    } else {
        Vec::new()
    }
}

#[tokio::main]
pub async fn main() {
    // register to CtrlC
    ctrlc::set_handler(move || {
        println!("exiting");
        fs::remove_file("yaaw.lock").expect("failed to remove lock file");
        std::process::exit(0);
    }).expect("failed to set Ctrl-C handler");

    let mut yelled_about_and_when = load_yaaw();

    if fs::metadata("yaaw.lock").is_ok() {
        println!("already running");
        return;
    }
    fs::write("yaaw.lock", "").expect("failed to write lock file");
    loop {
        println!("Querying servers");
        query_and_validate(&mut yelled_about_and_when, None).await;
        save_yaaw(&yelled_about_and_when);
        tokio::time::sleep(tokio::time::Duration::from_secs(QUERY_INTEVAL_MINUTES * 60)).await;
    }
}

#[derive(Deserialize, Serialize)]
struct YelledAboutAndWhen {
    server: String,
    when: SystemTime,
}

async fn query_and_validate(already_yelled_about: &mut Vec<YelledAboutAndWhen>, webhook: Option<&str>) {
    let data = fetch_compile_data::fetch_server_data().await;

    for (server, compile_data) in data {
        assert!(compile_data.revision_date.is_some());
        // revision date is in ISO 8601 format
        let revision_date = match compile_data
            .revision_date
            .unwrap()
            .parse::<chrono::DateTime<chrono::FixedOffset>>()
        {
            Ok(date) => date,
            Err(_) => {
                println!("{} has an invalid revision date", server);
                continue;
            }
        };

        let revision_date = revision_date.timestamp() as u64;
        if SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - revision_date
            > (ERROR_REVISION_DATE_UNCHANGED_FOR_HOURS * 60 * 60)
        {
            let yaaw = already_yelled_about.iter().position(|x| x.server == server);
            if yaaw.is_some() {
                let yaaw = yaaw.unwrap();
                let when = already_yelled_about[yaaw].when;
                let elapsed = SystemTime::now().duration_since(when).unwrap().as_secs();
                if elapsed < (24 * 60 * 60) {
                    continue;
                }
                already_yelled_about.remove(yaaw);
            }

            already_yelled_about.push(YelledAboutAndWhen {
                server: server.clone(),
                when: SystemTime::now(),
            });

            let message = format!(
                "{} has not updated in {} hours.\nThis error will not repeat until the server updates or 24 hours have passed.",
                server, ERROR_REVISION_DATE_UNCHANGED_FOR_HOURS
            );
            if let Some(webhook) = webhook {
                post_to_webhook(&message, webhook).await;
            }
            println!("{}", message);
        }
    }
}

async fn post_to_webhook(message: &str, webhook: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(webhook)
        .body(format!("{{\"content\":\"{}\"}}", message))
        .send()
        .await;
}
