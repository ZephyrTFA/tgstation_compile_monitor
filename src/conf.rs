#[derive(serde::Deserialize, Debug)]
pub struct TargetInfo {
    name_override: Option<String>,
    webhook_url: String,
    error_revision_date_unchanged_for_hours: u64,
    target_servers: Vec<String>,
}

impl TargetInfo {
    pub fn load_from(file: &str) -> Vec<Self> {
        let file = std::fs::read_to_string(file).expect(format!("failed to read from cfg file: {}", file).as_str());
        let data: Vec<Self> = serde_json::from_str(&file).unwrap();
        data
    }

    pub fn name_override(&self) -> &Option<String> {
        &self.name_override
    }

    pub fn webhook_url(&self) -> &str {
        &self.webhook_url
    }

    pub fn error_revision_date_unchanged_for_hours(&self) -> u64 {
        self.error_revision_date_unchanged_for_hours
    }

    pub fn target_servers(&self) -> &Vec<String> {
        &self.target_servers
    }
}
