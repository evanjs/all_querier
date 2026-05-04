use serde_json::Value;

use crate::WikidataClient;

pub async fn smoke_test() -> anyhow::Result<()> {
    let client = WikidataClient::new().await?;
    let res = client.userinfo().await?;

    print_pretty_json(&res)?;

    Ok(())
}

pub async fn smoke_test_entity_by_qid(qid: &str) -> anyhow::Result<()> {
    let client = WikidataClient::new().await?;
    let res = client.entity_by_qid(qid).await?;

    print_pretty_json(&res)?;

    Ok(())
}

fn print_pretty_json(value: &Value) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);

    Ok(())
}