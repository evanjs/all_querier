use serde_json::Value;

use crate::{
    WikidataClient,
    WikidataEntityLookupMode,
};
pub async fn smoke_test() -> anyhow::Result<()> {
    let client = WikidataClient::new().await?;
    let res = client.userinfo().await?;

    print_pretty_json(&res)?;

    Ok(())
}

pub async fn retrieve_entity_by_qid(qid: &str, cache_only: bool) -> anyhow::Result<()> {
    let client = if cache_only {
        WikidataClient::new_local_only().await?
    } else {
        WikidataClient::new().await?
    };

    let lookup_mode = if cache_only {
        WikidataEntityLookupMode::CacheOnly
    } else {
        WikidataEntityLookupMode::NetworkFallback
    };

    let res = client.entity_by_qid_with_mode(qid, lookup_mode).await?;

    print_pretty_json(&res)?;

    Ok(())
}

fn print_pretty_json(value: &Value) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);

    Ok(())
}