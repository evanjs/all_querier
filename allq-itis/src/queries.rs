use serde::{Deserialize, Serialize};
use sqlx::{Connection, Encode, Decode, FromRow, SqlitePool};
use sqlx::types::time::Date;

struct Vernacular {
    tsn: u32,
    vernacular_name: String,
    language: String,
    approved_ind: String,
    update_date: Date,
    vern_id: u32
}

#[derive(Encode, Decode, FromRow)]
#[derive(Deserialize, Serialize)]
pub struct CombinedResult {
    // vernacular
    pub tsn: u32,
    pub vernacular_name: String,
    pub language: String,
    pub approved_ind: String,
    // taxonomic unit
    pub complete_name: String,
    pub name_usage: String,
    pub parent_tsn: u32,
    pub rank_id: u32,
    pub credibility_rtng: String
}

pub async fn search_taxon_by_vernacular(
    conn: SqlitePool,
    vernacular_name: &str
) -> sqlx::Result<CombinedResult> {

    let results: CombinedResult = sqlx::query_as(r#"
SELECT
    v.tsn,
    v.vernacular_name,
    v.language,
    v.approved_ind,
    tu.complete_name,
    tu.name_usage,
    tu.parent_tsn,
    tu.rank_id,
    tu.credibility_rtng
FROM vernaculars v
JOIN taxonomic_units tu ON tu.tsn = v.tsn
WHERE lower(v.vernacular_name) = lower($1)
  AND v.language = 'English';
"#
    )
        .bind(vernacular_name.to_string())
        .fetch_one(&conn)
        .await?;

    Ok(results)

}