mod fetch_compile_data;

#[tokio::main]
pub async fn main() {
    let data = fetch_compile_data::fetch_server_data().await;
    for (server, compile_data) in data {
        println!(
            "{}: {} ({})",
            server,
            compile_data.revision.unwrap_or_default(),
            compile_data.revision_date.unwrap_or_default()
        );
    }
}
