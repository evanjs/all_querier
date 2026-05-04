use serde::{
    Deserialize,
    Serialize,
};

pub type Properties = Vec<Property>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    pub id: String,
    pub label: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub datatype_key: DatatypeKey,
    pub datatype_name: String,
    pub datatype_uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DatatypeKey {
    #[serde(rename = "CM")]
    CommonsMedia,

    #[serde(rename = "EI")]
    ExternalId,

    #[serde(rename = "ES")]
    EntitySchema,

    #[serde(rename = "GC")]
    GlobeCoordinate,

    #[serde(rename = "GS")]
    GeoShape,

    #[serde(rename = "M")]
    Math,

    #[serde(rename = "MN")]
    MusicalNotation,

    #[serde(rename = "MT")]
    MonolingualText,

    #[serde(rename = "Q")]
    Quantity,

    #[serde(rename = "S")]
    String,

    #[serde(rename = "T")]
    Time,

    #[serde(rename = "TD")]
    TabularData,

    #[serde(rename = "U")]
    Url,

    #[serde(rename = "WF")]
    WikibaseForm,

    #[serde(rename = "WI")]
    WikibaseItem,

    #[serde(rename = "WL")]
    WikibaseLexeme,

    #[serde(rename = "WP")]
    WikibaseProperty,

    #[serde(rename = "WS")]
    WikibaseSense,

    #[serde(rename = "UNKNOWN")]
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

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_wikibase_property_type(value))
    }
}