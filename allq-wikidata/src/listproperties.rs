use anyhow::Context;
use serde::Serialize;
use serde_json::Value;

use crate::WikidataClient;

const LIST_PROPERTIES_QUERY: &str = r#"
SELECT ?property ?propertyLabel ?propertyDescription ?propertyType
WHERE {
  ?property wikibase:propertyType ?propertyType .
  SERVICE wikibase:label { bd:serviceParam wikibase:language "[AUTO_LANGUAGE],en". }
}
ORDER BY ASC(?property)
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatatypeKey {
    CommonsMedia,
    ExternalId,
    EntitySchema,
    GlobeCoordinate,
    GeoShape,
    Math,
    MusicalNotation,
    MonolingualText,
    Quantity,
    String,
    Time,
    TabularData,
    Url,
    WikibaseForm,
    WikibaseItem,
    WikibaseLexeme,
    WikibaseProperty,
    WikibaseSense,
    Unknown,
}

impl DatatypeKey {
    pub fn as_key(self) -> &'static str {
        match self {
            DatatypeKey::CommonsMedia => "CM",
            DatatypeKey::ExternalId => "EI",
            DatatypeKey::EntitySchema => "ES",
            DatatypeKey::GlobeCoordinate => "GC",
            DatatypeKey::GeoShape => "GS",
            DatatypeKey::Math => "M",
            DatatypeKey::MusicalNotation => "MN",
            DatatypeKey::MonolingualText => "MT",
            DatatypeKey::Quantity => "Q",
            DatatypeKey::String => "S",
            DatatypeKey::Time => "T",
            DatatypeKey::TabularData => "TD",
            DatatypeKey::Url => "U",
            DatatypeKey::WikibaseForm => "WF",
            DatatypeKey::WikibaseItem => "WI",
            DatatypeKey::WikibaseLexeme => "WL",
            DatatypeKey::WikibaseProperty => "WP",
            DatatypeKey::WikibaseSense => "WS",
            DatatypeKey::Unknown => "UNKNOWN",
        }
    }

    pub fn as_name(self) -> &'static str {
        match self {
            DatatypeKey::CommonsMedia => "CommonsMedia",
            DatatypeKey::ExternalId => "ExternalId",
            DatatypeKey::EntitySchema => "EntitySchema",
            DatatypeKey::GlobeCoordinate => "GlobeCoordinate",
            DatatypeKey::GeoShape => "GeoShape",
            DatatypeKey::Math => "Math",
            DatatypeKey::MusicalNotation => "MusicalNotation",
            DatatypeKey::MonolingualText => "MonolingualText",
            DatatypeKey::Quantity => "Quantity",
            DatatypeKey::String => "String",
            DatatypeKey::Time => "Time",
            DatatypeKey::TabularData => "TabularData",
            DatatypeKey::Url => "Url",
            DatatypeKey::WikibaseForm => "WikibaseForm",
            DatatypeKey::WikibaseItem => "WikibaseItem",
            DatatypeKey::WikibaseLexeme => "WikibaseLexeme",
            DatatypeKey::WikibaseProperty => "WikibaseProperty",
            DatatypeKey::WikibaseSense => "WikibaseSense",
            DatatypeKey::Unknown => "Unknown",
        }
    }

    pub fn from_wikibase_property_type(value: &str) -> Self {
        let token = value
            .rsplit(['#', '/'])
            .next()
            .unwrap_or(value);

        match token {
            "CM" | "CommonsMedia" => DatatypeKey::CommonsMedia,
            "EI" | "ExternalId" => DatatypeKey::ExternalId,
            "ES" | "EntitySchema" => DatatypeKey::EntitySchema,
            "GC" | "GlobeCoordinate" => DatatypeKey::GlobeCoordinate,
            "GS" | "GeoShape" => DatatypeKey::GeoShape,
            "M" | "Math" => DatatypeKey::Math,
            "MN" | "MusicalNotation" => DatatypeKey::MusicalNotation,
            "MT" | "MonolingualText" | "Monolingualtext" => DatatypeKey::MonolingualText,
            "Q" | "Quantity" => DatatypeKey::Quantity,
            "S" | "String" => DatatypeKey::String,
            "T" | "Time" => DatatypeKey::Time,
            "TD" | "TabularData" => DatatypeKey::TabularData,
            "U" | "Url" | "URL" => DatatypeKey::Url,
            "WF" | "WikibaseForm" => DatatypeKey::WikibaseForm,
            "WI" | "WikibaseItem" => DatatypeKey::WikibaseItem,
            "WL" | "WikibaseLexeme" => DatatypeKey::WikibaseLexeme,
            "WP" | "WikibaseProperty" => DatatypeKey::WikibaseProperty,
            "WS" | "WikibaseSense" => DatatypeKey::WikibaseSense,
            _ => DatatypeKey::Unknown,
        }
    }
}

impl std::str::FromStr for DatatypeKey {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DatatypeKey::from_wikibase_property_type(s))
    }
}

impl Serialize for DatatypeKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_key())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PropertyRow {
    pub id: String,
    pub label: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "datatypeKey")]
    pub datatype_key: DatatypeKey,

    #[serde(rename = "datatypeName")]
    pub datatype_name: String,

    #[serde(rename = "datatypeUri")]
    pub datatype_uri: String,
}

pub async fn fetch_listproperties_rows_json() -> anyhow::Result<Vec<PropertyRow>> {
    let client = WikidataClient::new().await?;
    let res = client.sparql_query_json(LIST_PROPERTIES_QUERY).await?;

    parse_listproperties_rows(&res)
}

fn parse_listproperties_rows(value: &Value) -> anyhow::Result<Vec<PropertyRow>> {
    let bindings = value
        .pointer("/results/bindings")
        .and_then(Value::as_array)
        .context("SPARQL response is missing results.bindings")?;

    bindings
        .iter()
        .map(parse_listproperties_row)
        .collect()
}

fn parse_listproperties_row(binding: &Value) -> anyhow::Result<PropertyRow> {
    let property_uri = required_binding_value(binding, "property")?;
    let id = property_uri_to_id(property_uri)?;

    let label = required_binding_value(binding, "propertyLabel")?.to_string();
    let description = optional_binding_value(binding, "propertyDescription").map(ToString::to_string);

    let datatype_uri = required_binding_value(binding, "propertyType")?.to_string();
    let datatype_key = DatatypeKey::from_wikibase_property_type(&datatype_uri);

    Ok(PropertyRow {
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