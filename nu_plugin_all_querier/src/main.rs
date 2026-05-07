use nu_plugin::{MsgPackSerializer, Plugin, PluginCommand, serve_plugin};
use tracing_subscriber::EnvFilter;

mod commands;
pub use commands::*;

pub struct AllQuerierPlugin;

pub fn init_logging(verbose: bool) -> anyhow::Result<()> {
    let env_filter = if let Ok(rust_log) = std::env::var("RUST_LOG") {
        EnvFilter::try_new(rust_log)?
    } else if verbose {
        EnvFilter::try_new(
            "warn,allq_query=debug,allq_providers=debug,allq_wikidata=debug,nu_plugin_all_querier=debug",
        )?
    } else {
        EnvFilter::try_new("warn")?
    };

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .try_init();

    Ok(())
}

impl Plugin for AllQuerierPlugin {
    fn version(&self) -> String {
        // This automatically uses the version of your package from Cargo.toml as the plugin version
        // sent to Nushell
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            // Commands should be added here
            Box::new(QueryWikidata),
        ]
    }
}

fn main() {
    serve_plugin(&AllQuerierPlugin, MsgPackSerializer);
}