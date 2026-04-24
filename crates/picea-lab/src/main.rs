#[tokio::main]
async fn main() {
    if let Err(error) = picea_lab::cli::run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
