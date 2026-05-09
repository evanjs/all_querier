// Command modules should be added here
mod query_wikidata;
mod search;

// Command structs should be exported here
pub use query_wikidata::QueryWikidata;
pub use search::Search;
