mod app;
mod config;
mod domain;
mod repository;
mod server;

#[tokio::main]
async fn main() {
    if let Err(err) = server::run().await {
        eprintln!("BeamRS failed: {err}");
        std::process::exit(1);
    }
}
