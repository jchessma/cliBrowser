mod browser;
mod css;
mod dom;
mod js;
mod layout;
mod network;
mod renderer;
mod ui;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "clibrowser", about = "A full-featured CLI web browser")]
struct Args {
    /// URL to open on startup
    url: Option<String>,

    /// Use Chrome headless backend for JavaScript (requires Chrome installed)
    #[arg(long)]
    chrome: bool,

    /// Disable JavaScript execution
    #[arg(long)]
    no_js: bool,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "warn")]
    log: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| args.log.parse().unwrap_or_default()),
        )
        .with_target(false)
        .init();

    let js_backend = if args.no_js {
        js::Backend::None
    } else if args.chrome {
        js::Backend::Chrome
    } else {
        js::Backend::QuickJs
    };

    let start_url = args.url.unwrap_or_else(|| "about:home".to_string());

    ui::run(start_url, js_backend).await
}
