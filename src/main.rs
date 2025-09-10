//! Urpo CLI entry point.

use urpo_lib::cli::{self, Cli};
use urpo_lib::core::Result;

// EXTREME PERFORMANCE: Use mimalloc for blazing fast memory allocation
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let cli = Cli::parse_args();
    
    // Execute the command
    cli::execute(cli).await
}