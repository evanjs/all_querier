#[tokio::main]
async fn main() {
    if let Err(error) = allq_wikidata::smoke_test_entity_by_qid("Q105337231").await {
        eprintln!("error: {error:#}");

        let mut source = error.source();
        while let Some(error) = source {
            eprintln!("caused by: {error}");
            source = error.source();
        }
    }
}
