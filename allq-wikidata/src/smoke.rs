use serde_json::Value;

use crate::{
    WikidataClient,
    WikidataEntityLookupMode,
    add_external_links_to_wbgetentities_response,
};


pub async fn smoke_test() -> anyhow::Result<()> {
    let client = WikidataClient::new().await?;
    let res = client.userinfo().await?;

    print_pretty_json(&res)?;

    Ok(())
}

pub async fn retrieve_entity_by_qid(qid: &str, cache_only: bool) -> anyhow::Result<()> {
    retrieve_entity_by_qid_with_options(qid, cache_only, false).await
}

pub async fn retrieve_entity_by_qid_with_options(
    qid: &str,
    cache_only: bool,
    force_fetch: bool,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        !(cache_only && force_fetch),
        "--cache-only and --force-fetch cannot be used together"
    );

    let client = if cache_only {
        WikidataClient::new_local_only().await?
    } else {
        WikidataClient::new().await?
    };

    let lookup_mode = if cache_only {
        WikidataEntityLookupMode::CacheOnly
    } else if force_fetch {
        WikidataEntityLookupMode::ForceFetch
    } else {
        WikidataEntityLookupMode::NetworkFallback
    };

    let mut res = client.entity_by_qid_with_mode(qid, lookup_mode).await?;
    add_external_links_to_wbgetentities_response(&mut res, &client, lookup_mode).await?;

    print_pretty_json(&res)?;

    Ok(())
}

fn print_pretty_json(value: &Value) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);

    Ok(())
}