use crate::dynamodb::utils::{describe_table, print_dynamo_items};
use crate::error::CustomError;
use clap::Subcommand;

use crate::cli::get_aws_client_config;
use aws_sdk_dynamodb::{
    Client as DynamoClient,
    error::SdkError,
    operation::scan::ScanError,
    types::{AttributeValue, KeyType},
};
use std::collections::HashMap;

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
        #[arg(short, long)]
        pk: String,
        #[arg(short, long)]
        sk: Option<String>,
    },
}
