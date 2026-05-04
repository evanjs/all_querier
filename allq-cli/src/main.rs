use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "allq")]
#[command(about = "Query all the things")]
struct Cli {
    #[arg(short, long)]
    qid: String,

    #[arg(long, help = "Only read from the local Wikidata cache; do not call the Wikidata API")]
    cache_only: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(error) = allq_wikidata::retrieve_entity_by_qid(&cli.qid, cli.cache_only).await {
        eprintln!("error: {error:#}");

        let mut source = error.source();
        while let Some(error) = source {
            eprintln!("caused by: {error}");
            source = error.source();
        }

        std::process::exit(1);
    }
}