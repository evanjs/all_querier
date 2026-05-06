use nu_plugin::{MsgPackSerializer, Plugin, PluginCommand, serve_plugin};

mod commands;
pub use commands::*;

pub struct AllQuerierPlugin;

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
