use std::{
    env, fs, process,
    time::{SystemTime, UNIX_EPOCH},
};

use sysinfo::{Pid, System};
use yaaw::{load_yaaw, save_yaaw, YelledAboutAndWhen};

use crate::webhook::post_to_webhook;

mod conf;
mod fetch_compile_data;
mod webhook;
mod yaaw;

const QUERY_INTEVAL_MINUTES: u64 = 60;

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
            continue;
        }

        let mut fail_reason = None;

        let revision_date = compile_data.revision_timestamp();
        if let Err(reason) = revision_date {
            fail_reason = Some(format!(
                "Revision date information was invalid: `{reason}`."
            ))
        } else {
            let revision_timestamp = revision_date.unwrap();
            if compile_data.is_extended_round() {
                println!("{} is experiencing an extended round, skipping.", server);
                continue;
            }

            let elapsed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - revision_timestamp;

            let elapsed_threshold = cfg.error_revision_date_unchanged_for_hours();
            let last_updated_hours_ago = elapsed / 3600;
            if elapsed > (elapsed_threshold * 60 * 60) {
                fail_reason = Some(format!("Server has not updated in `{elapsed_threshold}` hours; last updated on `{revision_timestamp} ({last_updated_hours_ago}h ago)`."));
            }
        }

        if let Some(fail_reason) = fail_reason {
            let yaaw = already_yelled_about
                .iter()
                .position(|x| x.server() == server);
            if yaaw.is_some() {
                let yaaw = yaaw.unwrap();
                let when = already_yelled_about[yaaw].when();
                let time_since_last = SystemTime::now().duration_since(when).unwrap().as_secs();
                if time_since_last < (24 * 60 * 60) {
                    continue;
                }
                already_yelled_about.remove(yaaw);
            }

            already_yelled_about.push(YelledAboutAndWhen::new(
                cfg.webhook_url(),
                &server,
                SystemTime::now(),
            ));

            let mut message = format!(
                "`{}` failed validation.\n{fail_reason}\nThis error will not repeat until the server updates or 24 hours have passed.",
                &server,
            );
            if let Some(ping_role_id) = cfg.ping_role_id() {
                message = format!("<@&{}>\n{}", ping_role_id, message);
            }

            post_to_webhook(&message, cfg).await;
            println!("{}", &message);
            println!("sent to webhook, failed to update");
        }
    }
}
