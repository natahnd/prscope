//! Binary entry point for `pr-reviews`.

use pr_reviews::cli::run;

#[tokio::main]
async fn main() {
    run().await;
}
