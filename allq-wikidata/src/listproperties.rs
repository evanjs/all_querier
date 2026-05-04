use anyhow::Context;
use serde_json::Value;

use crate::{
    Property,
    DatatypeKey,
    WikidataClient,
};

const LIST_PROPERTIES_QUERY: &str = r#"
SELECT ?property ?propertyLabel ?propertyDescription ?propertyType
WHERE {
  ?property wikibase:propertyType ?propertyType .
  SERVICE wikibase:label { bd:serviceParam wikibase:language "[AUTO_LANGUAGE],en". }
}
ORDER BY ASC(?property)
"#;

pub async fn fetch_listproperties_rows_json() -> anyhow::Result<Vec<Property>> {
    let client = WikidataClient::new().await?;
    let res = client.sparql_query_json(LIST_PROPERTIES_QUERY).await?;

    parse_listproperties_rows(&res)
}

fn parse_listproperties_rows(value: &Value) -> anyhow::Result<Vec<Property>> {
    let bindings = value
        .pointer("/results/bindings")
        .and_then(Value::as_array)
        .context("SPARQL response is missing results.bindings")?;

    bindings
        .iter()
        .map(parse_listproperties_row)
        .collect()
}

fn parse_listproperties_row(binding: &Value) -> anyhow::Result<Property> {
    let property_uri = required_binding_value(binding, "property")?;
    let id = property_uri_to_id(property_uri)?;

    let label = required_binding_value(binding, "propertyLabel")?.to_string();
    let description = optional_binding_value(binding, "propertyDescription").map(ToString::to_string);

    let datatype_uri = required_binding_value(binding, "propertyType")?.to_string();
    let datatype_key = DatatypeKey::from_wikibase_property_type(&datatype_uri);

    Ok(Property {
        id,
        label,
        description,
        datatype_key,
        datatype_name: datatype_key.as_name().to_string(),
        datatype_uri,
    })
}

fn required_binding_value<'a>(binding: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    binding
        .get(key)
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
        .with_context(|| format!("SPARQL row is missing binding value: {key}"))
}

fn optional_binding_value<'a>(binding: &'a Value, key: &str) -> Option<&'a str> {
    binding
        .get(key)
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
}

fn property_uri_to_id(uri: &str) -> anyhow::Result<String> {
    let id = uri
        .rsplit('/')
        .next()
        .context("property URI has no path segment")?;

    anyhow::ensure!(
        id.starts_with('P'),
        "expected Wikidata property ID starting with P, got: {id}"
    );

    Ok(id.to_string())
}