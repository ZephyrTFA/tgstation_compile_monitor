use http2byond::{send_byond, ByondTopicValue};
use reqwest::Url;
use serde::Deserialize;
use std::{collections::HashMap, fmt::format, str::FromStr};

#[derive(Deserialize)]
pub struct GlobalCompileData {
    pub refreshtime: u64,
    #[serde(flatten)]
    pub servers: HashMap<String, ServerCompileData>,
}

// there is a lot more data in the json response, but we only care about these fields
#[derive(Deserialize)]
pub struct ServerCompileData {
    pub revision: Option<String>,
    pub revision_date: Option<String>,
    error: Option<bool>,
    serverdata: ServerData,
}

#[derive(Deserialize)]
struct ServerData {
    address: String,
    port: u16,
}

#[derive(Deserialize)]
struct TopicResponse {
    revision_date: String,
}

const TGSTATION_ENDPOINT: &str = "https://tgstation13.download/serverinfo.json";
async fn fetch_revision_from_world_topic(data: &mut ServerCompileData) -> bool {
    let server_addr = format!("http://{}/", data.serverdata.address);
    let socket_addr = Url::from_str(&server_addr)
        .expect("failed to parse url")
        .socket_addrs(|| Some(data.serverdata.port))
        .expect("failed to resolve url")
        .pop()
        .expect("failed to resolve url");
    if let Ok(ByondTopicValue::String(respone_json)) = send_byond(&socket_addr, "status") {
        let response: TopicResponse =
            serde_json::from_str(&respone_json).expect("failed to parse response");
        data.revision_date = Some(response.revision_date);
        return true;
    }
    false
}

pub async fn fetch_server_data() -> HashMap<String, ServerCompileData> {
    let response = reqwest::get(TGSTATION_ENDPOINT).await.unwrap();
    let mut json: GlobalCompileData =
        serde_json::from_str(&response.text().await.expect("failed to fetch data"))
            .expect("failed to parse data");
    for server in json.servers.values_mut() {
        if server.error.is_some_and(|x| x) {
            continue;
        }
        if server.revision_date.is_none()
            || server.revision_date.as_ref().is_some_and(|x| x.is_empty())
        {
            if !fetch_revision_from_world_topic(server).await {
                println!("failed to fetch revision for {}", server.serverdata.address)
            }
        }
    }
    json.servers
        .into_iter()
        .filter(|(_, x)| x.error.is_none())
        .collect()
}
