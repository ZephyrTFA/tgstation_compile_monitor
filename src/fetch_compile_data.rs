use serde::Deserialize;
use std::collections::HashMap;

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
}

const TGSTATION_ENDPOINT: &str = "https://tgstation13.download/serverinfo.json";

pub async fn fetch_server_data() -> HashMap<String, ServerCompileData> {
    let response = reqwest::get(TGSTATION_ENDPOINT).await.unwrap();
    let json: GlobalCompileData =
        serde_json::from_str(&response.text().await.expect("failed to fetch data"))
            .expect("failed to parse data");
    json.servers
        .into_iter()
        .filter(|(_, x)| x.error.is_none())
        .collect()
}
