use std::{
    env, fs, process,
    time::{SystemTime, UNIX_EPOCH},
};

use conf::TargetInfo;
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};

mod conf;
mod fetch_compile_data;

const QUERY_INTEVAL_MINUTES: u64 = 60;

fn save_yaaw(yaaw: &Vec<YelledAboutAndWhen>) {
    let webhook_url = yaaw.first().map(|x| x.webhook.clone());
    if webhook_url.is_none() {
        println!("no data to save");
        return;
    }

    let webhook_id = webhook_url_to_id(&webhook_url.unwrap());
    // ensure the directory exists
    fs::create_dir_all("./yaaw").expect("failed to create yaaw directory");
    serde_json::to_writer(
        &fs::File::create(format!("./yaaw/{}.json", webhook_id))
            .expect("failed to create yaaw file."),
        yaaw,
    )
    .expect("failed to write yaaw file.");
}

fn load_yaaw(webhook_url: &str) -> Vec<YelledAboutAndWhen> {
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

#[tokio::main]
pub async fn main() {
    // register to CtrlC
    ctrlc::set_handler(move || {
        println!("exiting");
        fs::remove_file("yaaw.lock").expect("failed to remove lock file");
        std::process::exit(0);
    })
    .expect("failed to set Ctrl-C handler");

    println!(
        "Working directory: {}",
        std::env::current_dir().unwrap().display()
    );
    if let Ok(contents) = fs::read_to_string("yaaw.lock") {
        let them_pid = Pid::from_u32(contents.parse::<u32>().expect("failed to parse lock file"));
        let me_pid = Pid::from_u32(process::id());
        let mut system = System::new();
        system.refresh_process(them_pid);
        system.refresh_process(me_pid);

        let them = system.process(them_pid);
        let me = system.process(me_pid);
        if them.is_some_and(|x| x.name() == me.expect("failed to get self process info").name()) {
            println!("Yaaw is already running");
            process::exit(1);
        }
        println!("ignoring yaaw.lock: process {} is not running", them_pid);
    }
    fs::write("yaaw.lock", process::id().to_string()).expect("failed to write lock file");

    let webhook_url = env::var("DISCORD_WEBHOOK_URL").ok();
    if webhook_url.is_none() {
        println!("DISCORD_WEBHOOK_URL not set!");
    }

    let cfg = conf::TargetInfo::load_from("config.json");

    loop {
        println!("Querying servers");
        for target in &cfg {
            let webhook_url = target.webhook_url();
            let mut yaaw = load_yaaw(webhook_url);
            query_and_validate(&mut yaaw, target).await;
            save_yaaw(&yaaw);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(QUERY_INTEVAL_MINUTES * 60)).await;
    }
}

#[derive(Deserialize, Serialize)]
struct YelledAboutAndWhen {
    webhook: String,
    server: String,
    when: SystemTime,
}

async fn query_and_validate(
    already_yelled_about: &mut Vec<YelledAboutAndWhen>,
    cfg: &conf::TargetInfo,
) {
    println!("Fetching server data for {}", cfg.webhook_url());
    let data = fetch_compile_data::fetch_server_data().await;

    for wanted in cfg.target_servers() {
        if !data.contains_key(wanted) {
            println!("{} was not found in server information", wanted);
        }
    }

    for (server, compile_data) in data {
        if !cfg.target_servers().contains(&server) {
            println!("{} is not a target server", server);
            continue;
        }

        if compile_data.revision_date.is_none() {
            println!("{} has no revision date", server);
            continue;
        }
        println!(
            "{} - {}",
            server,
            compile_data.revision_date.as_ref().unwrap()
        );

        assert!(compile_data.revision_date.is_some());
        // revision date is in ISO 8601 format
        let revision_date = match compile_data
            .revision_date
            .as_ref()
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
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - revision_date;

        let elapsed_threshold = cfg.error_revision_date_unchanged_for_hours();
        if elapsed > (elapsed_threshold * 60 * 60) {
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
                webhook: cfg.webhook_url().to_string(),
                server: server.clone(),
                when: SystemTime::now(),
            });

            let message = format!(
                "`{}` has not updated in `{}` hours.\nIt last updated on `{} ({}h ago)`.\nThis error will not repeat until the server updates or 24 hours have passed.",
                server,
                elapsed_threshold,
                compile_data.revision_date.as_ref().unwrap(),
                elapsed / 3600,
            );
            post_to_webhook(&message, cfg).await;
            println!("sent to webhook, failed to update");
        }
    }
}

#[derive(Serialize)]
struct WebhookPostData {
    content: String,
    username: Option<String>,
}

async fn post_to_webhook(message: &str, cfg: &TargetInfo) {
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
