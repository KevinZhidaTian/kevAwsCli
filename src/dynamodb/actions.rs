use clap::Subcommand;

#[derive(Debug, Clone, Subcommand)]
pub enum DynamoDBAction {
    ListSamples {
        #[arg(short, long)]
        table_name: String,
    },
    EmptyTable {
        #[arg(short, long)]
        table_name: String,
    },
    ScanTable {
        #[arg(short, long)]
        table_name: String,
    },
    Query {
        #[arg(short, long)]
        table_name: String,
        #[arg(long)]
        pk: String,
        #[arg(long)]
        sk: Option<String>,
    },
}
