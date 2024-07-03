use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct GlobalCompileData {
    #[serde(rename = "refreshtime")]
    _refreshtime: u64,
    #[serde(flatten)]
    servers: HashMap<String, ServerCompileData>,
}

// there is a lot more data in the json response, but we only care about these fields
#[derive(Deserialize)]
pub struct ServerCompileData {
    revision_date: Option<String>,
    // in deci-secs
    round_duration: Option<u64>,
    #[serde(rename = "serverdata")]
    server_data: ServerData,
    error: Option<bool>,
}

impl ServerCompileData {
    pub fn revision_date(&self) -> Option<&String> {
        self.revision_date.as_ref()
    }

    pub fn is_extended_round(&self) -> bool {
        self.round_duration.is_some_and(|x| x > 2 * 60 * 60 * 10)
    }
}

#[derive(Deserialize)]
struct ServerData {
    #[serde(rename = "dbname")]
    db_name: Option<String>,
}

const TGSTATION_ENDPOINT: &str = "https://tgstation13.download/serverinfo.json";

pub async fn fetch_server_data() -> HashMap<String, ServerCompileData> {
    let response = reqwest::get(TGSTATION_ENDPOINT).await.unwrap();
    let json: GlobalCompileData =
        serde_json::from_str(&response.text().await.expect("failed to fetch data"))
            .expect("failed to parse data");
    json.servers
        .into_iter()
        .filter(|(_, x)| {
            if x.error.is_some() {
                return false;
            }
            true
        })
        .map(|(_, v)| (v.server_data.db_name.as_ref().unwrap().clone(), v))
        .map(|(name, mut data)| {
            if data.revision_date.as_ref().is_some_and(|rd| rd.is_empty()) {
                data.revision_date = None;
            }
            (name, data)
        })
        .collect()
}
