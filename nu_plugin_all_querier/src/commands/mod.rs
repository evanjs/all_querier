// Command modules should be added here
mod list_providers;
mod query_wikidata;
mod search;
mod search_game;

// Command structs should be exported here
pub use list_providers::ListProviders;
pub use query_wikidata::QueryWikidata;
pub use search::Search;
pub use search_game::SearchGame;
