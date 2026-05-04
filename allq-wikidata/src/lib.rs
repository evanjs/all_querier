mod client;
mod smoke;

pub use client::{
    ENTITY_QUERY_PROPS, WIKIDATA_API_URL, WikidataClient, wikidata_api,
};
pub use smoke::{
    smoke_test, smoke_test_entity_by_qid,
};