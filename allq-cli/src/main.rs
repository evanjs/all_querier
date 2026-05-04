use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "allq")]
#[command(about = "Query all the things")]
struct Cli {
    #[arg(short, long)]
    qid: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(error) = allq_wikidata::smoke_test_entity_by_qid(&cli.qid).await {
        eprintln!("error: {error:#}");

        let mut source = error.source();
        while let Some(error) = source {
            eprintln!("caused by: {error}");
            source = error.source();
        }
    }
}