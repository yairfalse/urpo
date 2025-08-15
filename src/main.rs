//! Urpo CLI entry point.

use urpo::cli::{self, Cli};
use urpo::core::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let cli = Cli::parse_args();
    
    // Execute the command
    cli::execute(cli).await
}