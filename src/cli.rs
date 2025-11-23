use aws_config::{
    BehaviorVersion, SdkConfig, meta::region::RegionProviderChain,
    profile::ProfileFileCredentialsProvider,
};
use clap::{Parser, Subcommand};

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
}

#[derive(Debug, Clone, Subcommand)]
pub enum AWSService {
    DynamoDB {
        #[command(subcommand)]
        action: DynamoDBAction,
    },
    S3 {
        #[arg(short, long)]
        bucket_name: String,
    },
}

#[derive(Parser, Debug)]
#[command(author="Kevin Tian", version, about = "Cli Tool for some AWS services", long_about = None)]
pub struct Args {
    #[arg(
        global = true,
        short,
        long,
        default_value = "default",
        help = "The AWS profile to use"
    )]
    pub profile: Option<String>,

    #[command(subcommand)]
    pub aws_service: AWSService,
}

pub async fn get_aws_client_config(profile: String) -> SdkConfig {
    let region_provider = RegionProviderChain::default_provider().or_else("eu-west-2");

    let credentials_provider = ProfileFileCredentialsProvider::builder()
        .profile_name(profile)
        .build();

    aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .credentials_provider(credentials_provider)
        .load()
        .await
}
