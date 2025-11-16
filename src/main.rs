use std::{
    collections::{BTreeSet, HashMap},
    fmt,
};

use aws_config::{
    BehaviorVersion, SdkConfig, meta::region::RegionProviderChain,
    profile::ProfileFileCredentialsProvider,
};
use aws_sdk_dynamodb::{
    Client as DynamoClient, Error as DynamoError,
    error::SdkError,
    operation::{describe_table::DescribeTableOutput, scan::ScanError},
    types::{AttributeValue, KeyType},
};
use clap::{Parser, Subcommand};
use comfy_table::{Cell, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};

#[derive(Debug)]
pub enum CustomError {
    DynamoError(DynamoError),
    UnexpectedActionVariant(String),
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CustomError::UnexpectedActionVariant(msg) => {
                write!(f, "Unexpected action variant: {}", msg)
            }
            CustomError::DynamoError(e) => write!(f, "AWS DynamoDB error: {}", e),
        }
    }
}
impl From<DynamoError> for CustomError {
    fn from(e: DynamoError) -> Self {
        CustomError::DynamoError(e)
    }
}
impl std::error::Error for CustomError {}

#[derive(Debug, Clone, Subcommand)]
enum DynamoDBAction {
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
enum AWSService {
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
#[command(author="Kevin Tian" ,version, about = "Cli Tool for some AWS services", long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "default",
        help = "The AWS profile to use"
    )]
    profile: Option<String>,

    #[command(subcommand)]
    aws_service: AWSService,
}

async fn get_aws_client_config(profile: String) -> SdkConfig {
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

#[derive(Debug)]
pub struct DynamicTable {
    pk: String,
    sk: Option<String>,
    headers: BTreeSet<String>,
    rows: Vec<HashMap<String, String>>,
}

impl DynamicTable {
    pub fn new(primary_key: String, sort_key: Option<String>) -> Self {
        DynamicTable {
            pk: primary_key,
            sk: sort_key,
            headers: BTreeSet::new(),
            rows: Vec::new(),
        }
    }

    pub fn add_row<K: Into<String>, V: Into<String>>(&mut self, item: HashMap<K, V>) {
        let mut row = HashMap::new();

        for (k, v) in item {
            let key = k.into();
            self.headers.insert(key.clone());
            row.insert(key, v.into());
        }

        self.rows.push(row);
    }

    fn ordered_header(&mut self) -> Vec<String> {
        let mut other_headers: Vec<String> = self
            .headers
            .iter()
            .filter(|header| {
                header.as_str() != self.pk
                    && header.as_str() != self.sk.as_ref().unwrap_or(&String::from("Null"))
            })
            .cloned()
            .collect();

        other_headers.sort();

        let mut ordered_header = Vec::new();

        if self.headers.contains(&self.pk) {
            ordered_header.push(self.pk.to_string());
        }

        if self
            .headers
            .contains(self.sk.as_ref().unwrap_or(&String::from("Null")))
        {
            ordered_header.push(self.sk.clone().unwrap_or(String::from("Null")));
        }

        ordered_header.extend(other_headers);

        ordered_header
    }

    pub fn to_table(&mut self) -> Table {
        let mut table = Table::new();

        let headers: Vec<String> = self.ordered_header();

        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
            .set_header(headers.clone());

        self.rows.iter().for_each(|row| {
            let mut cells = Vec::new();

            for header in &headers {
                let value = row.get(header).cloned().unwrap_or_default();
                cells.push(Cell::new(value));
            }

            table.add_row(cells);
        });

        table
    }
}

fn format_attribute_value(value: &AttributeValue) -> String {
    match value {
        AttributeValue::S(s) => s.clone(),
        AttributeValue::N(n) => n.clone(),
        AttributeValue::Bool(b) => b.to_string(),
        AttributeValue::Null(_) => "NULL".to_string(),
        AttributeValue::B(b) => format!("<Binary: {} bytes>", b.as_ref().len()),
        AttributeValue::Ss(ss) => {
            format!("[{}]", ss.iter().cloned().collect::<Vec<_>>().join(", "))
        }
        AttributeValue::Ns(ns) => {
            format!("[{}]", ns.iter().cloned().collect::<Vec<_>>().join(", "))
        }
        AttributeValue::Bs(bs) => format!("<BinarySet: {} items>", bs.len()),
        AttributeValue::L(list) => {
            let items: Vec<String> = list.iter().map(format_attribute_value).collect();
            format!("[{}]", items.join(", "))
        }
        AttributeValue::M(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_attribute_value(v)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
        _ => format!("{:?}", value),
    }
}

fn print_dynamo_items(
    items: &Vec<HashMap<String, AttributeValue>>,
    table_desc: &DescribeTableOutput,
) {
    if items.is_empty() {
        return;
    }

    // println!("{:#?}", table_desc.table);

    let pk = table_desc
        .table
        .as_ref()
        .and_then(|t| t.key_schema.as_ref())
        .and_then(|ks| {
            ks.iter()
                .find(|schema| schema.key_type == KeyType::Hash)
                .map(|schema| schema.attribute_name.clone())
        });

    let sk = table_desc
        .table
        .as_ref()
        .and_then(|t| t.key_schema.as_ref())
        .and_then(|ks| {
            ks.iter()
                .find(|schema| schema.key_type == KeyType::Range)
                .map(|schema| schema.attribute_name.clone())
        });

    let mut dynamic_table = DynamicTable::new(pk.unwrap(), sk);

    for item in items {
        let mut row = HashMap::new();
        for (key, _) in item {
            let cell_text = item
                .get(key)
                .map(format_attribute_value)
                .unwrap_or_default();
            row.insert(key, cell_text);
        }

        dynamic_table.add_row(row);
    }

    // println!("{:?}", table.rows);

    let print_table = dynamic_table.to_table();

    println!("{print_table}");
}

async fn describe_table(
    profile: String,
    table_name: &String,
) -> Result<DescribeTableOutput, CustomError> {
    let config = get_aws_client_config(profile).await;
    let db_client = DynamoClient::new(&config);

    let describe_table_response = db_client
        .describe_table()
        .table_name(table_name)
        .send()
        .await
        .map_err(|e| CustomError::DynamoError(e.into()))?;

    Ok(describe_table_response)
}

impl DynamoDBAction {
    async fn list_samples(
        &self,
        profile: Option<String>,
    ) -> Result<Vec<HashMap<std::string::String, AttributeValue>>, CustomError> {
        let DynamoDBAction::ListSamples { table_name } = self else {
            return Err(CustomError::UnexpectedActionVariant(
                "Unexpected action for list_samples".to_string(),
            ));
        };
        let config = get_aws_client_config(profile.clone().unwrap_or("default".to_string())).await;
        let db_client = DynamoClient::new(&config);

        let response = db_client
            .scan()
            .table_name(table_name)
            .limit(10)
            .send()
            .await;

        let scan_output = match response {
            Ok(output) => output,
            Err(e) => {
                if let SdkError::ServiceError(service_error) = &e {
                    if let ScanError::ResourceNotFoundException(resource_not_found_error) =
                        service_error.err()
                    {
                        println!(
                            "Table {:?} not found: {}",
                            table_name,
                            resource_not_found_error.message().unwrap_or("No message")
                        );
                        return Ok(Vec::new());
                    }
                }
                return Err(CustomError::DynamoError(e.into()));
            }
        };

        if scan_output.items().is_empty() {
            println!("No items found in table {:?}.", table_name);
            return Ok(Vec::new());
        }

        println!("Sample items from table {:?}: \n", table_name);
        let samples: Vec<HashMap<std::string::String, AttributeValue>> =
            scan_output.items().into_iter().cloned().collect();

        let describe_table_response = describe_table(profile.clone().unwrap(), table_name).await?;
        print_dynamo_items(&samples, &describe_table_response);

        Ok(samples)
    }

    async fn scan_table(
        &self,
        profile: Option<String>,
    ) -> Result<Vec<HashMap<std::string::String, AttributeValue>>, CustomError> {
        let table_name = match self {
            DynamoDBAction::ScanTable { table_name } => Some(table_name),
            DynamoDBAction::EmptyTable { table_name } => Some(table_name),
            _ => None,
        };

        let table_name = match table_name {
            Some(name) => name,
            None => {
                return Err(CustomError::UnexpectedActionVariant(
                    "scan_table called on unsupported variant".to_string(),
                ));
            }
        };

        let config = get_aws_client_config(profile.clone().unwrap_or("default".to_string())).await;
        let db_client = DynamoClient::new(&config);

        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;
        let mut all_items: Vec<HashMap<String, AttributeValue>> = Vec::new();

        loop {
            let mut scan_request = db_client.scan().table_name(table_name);

            match last_evaluated_key {
                None => {}
                key => {
                    scan_request = scan_request.set_exclusive_start_key(key);
                }
            }

            let scan_result = scan_request
                .send()
                .await
                .map_err(|e| CustomError::DynamoError(e.into()));

            match scan_result {
                Ok(output) => {
                    output
                        .items()
                        .iter()
                        .for_each(|item| all_items.push(item.clone()));

                    if output.last_evaluated_key().is_none() {
                        break;
                    } else {
                        last_evaluated_key = output.last_evaluated_key().cloned();
                    }
                }
                Err(e) => return Err(e),
            }
        }

        let describe_table_response = describe_table(profile.clone().unwrap(), table_name).await?;
        print_dynamo_items(&all_items, &describe_table_response);
        Ok(all_items)
    }

    async fn empty_table(&self, profile: Option<String>) -> Result<(), CustomError> {
        let DynamoDBAction::EmptyTable { table_name } = self else {
            return Err(CustomError::UnexpectedActionVariant(
                "Unexpected action for empty_table".to_string(),
            ));
        };

        let config = get_aws_client_config(profile.clone().unwrap_or("default".to_string())).await;
        let db_client = DynamoClient::new(&config);

        let describe_table_response = describe_table(profile.clone().unwrap(), table_name).await?;

        let key_schema = match describe_table_response.table {
            Some(description) => description.key_schema,
            None => None,
        };

        let pk = key_schema.as_ref().and_then(|schema| {
            schema
                .iter()
                .find(|element| element.key_type == KeyType::Hash)
                .map(|element| element.attribute_name.clone())
        });

        let all_items = self.scan_table(profile.clone()).await?;

        for item in all_items {
            let key_value = item.get(pk.as_ref().unwrap()).cloned().unwrap();
            db_client
                .delete_item()
                .table_name(table_name)
                .key(pk.as_ref().unwrap(), key_value)
                .send()
                .await
                .map_err(|e| CustomError::DynamoError(e.into()))?;
        }

        println!("Successfully emptied table: {}", table_name);
        Ok(())
    }
}

impl AWSService {
    async fn execute(&self, profile: Option<String>) -> Result<(), CustomError> {
        match self {
            AWSService::DynamoDB { action } => match action {
                DynamoDBAction::ListSamples { .. } => {
                    action.list_samples(profile).await?;
                }
                DynamoDBAction::EmptyTable { .. } => {
                    action.empty_table(profile).await?;
                }
                DynamoDBAction::ScanTable { .. } => {
                    action.scan_table(profile).await?;
                }
            },
            AWSService::S3 { bucket_name } => {
                println!("S3 Bucket Name: {}", bucket_name);
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), CustomError> {
    let args = Args::parse();
    // println!("Received Args:\n {:?}\n", args);

    args.aws_service.execute(args.profile).await?;

    Ok(())
}
