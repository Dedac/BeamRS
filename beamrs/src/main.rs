#[tokio::main]
async fn main() {
    if let Err(err) = beamrs::server::run().await {
        eprintln!("BeamRS failed: {err:#}");
        std::process::exit(1);
    }
}
