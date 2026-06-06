use nu_plugin::{MsgPackSerializer, Plugin, PluginCommand, serve_plugin};
use tracing_subscriber::EnvFilter;

mod commands;
pub use commands::*;

pub struct AllQuerierPlugin;

// TODO: consider moving these to something more structured
//  like an enum with variants, struct with impl, etc.
#[allow(unused)]
fn get_author_info() -> (&'static str, &'static str) {
    let author = env!("CARGO_PKG_AUTHORS")
        .split(':')
        .next()
        .unwrap_or(env!("CARGO_PKG_AUTHORS"));

    let (author_name, author_email) = author
        .split_once('<')
        .and_then(|(name, rest)| {
            rest.split_once('>')
                .map(|(email, _)| (name.trim(), email.trim()))
        })
        .unwrap_or((author.trim(), author.trim()));

    (author_name, author_email)
}

#[allow(unused)]
fn user_agent_name() -> String {
    let user_agent = format!(
        "{}/{} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        get_author_info().0,
    );

    user_agent.to_string()
}

#[allow(unused)]
fn user_agent_email() -> String {
    let user_agent = format!(
        "{}/{} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        get_author_info().1,
    );

    user_agent.to_string()
}

#[allow(unused)]
fn user_agent_name_email() -> String {
    let user_agent = format!(
        "{}/{} ( {} <{}> )",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        get_author_info().0,
        get_author_info().1,
    );

    user_agent.to_string()
}

pub fn init_logging(verbose: bool) -> anyhow::Result<()> {
    let env_filter = if let Ok(rust_log) = std::env::var("RUST_LOG") {
        EnvFilter::try_new(rust_log)?
    } else if verbose {
        EnvFilter::try_new(
            "warn,allq_query=debug,allq_providers=debug,allq_wikidata=debug,allq_core=debug,allq_mal=debug,allq_pcgw=debug,allq_musicbrainz=debug,allq_jikan=debug,allq_anilist=debug,allq_itis=debug,allq-igdb=debug,reqwest=debug,nu_plugin_all_querier=debug"
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
            Box::new(ListProviders),
            Box::new(QueryWikidata),
            Box::new(Search),
        ]
    }
}

fn main() {
    let _ = dotenvy::dotenv();
    
    // We pass true for verbose logging only if NU_PLUGIN_ALL_QUERIER_DEBUG is set
    // otherwise let default configuration handle it
    serve_plugin(&AllQuerierPlugin, MsgPackSerializer);
}