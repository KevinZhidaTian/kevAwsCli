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

impl DynamoDBAction {
    pub async fn list_samples(
        &self,
        profile: Option<String>,
    ) -> Result<Vec<HashMap<std::string::String, AttributeValue>>, CustomError> {
        let DynamoDBAction::ListSamples { table_name } = self else {
            return Err(CustomError::UnexpectedActionVariant(
                "Unexpected action for list_samples".to_string(),
            ));
        };
        let config = get_aws_client_config(&profile.clone().unwrap_or("default".to_string())).await;
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

        let samples: Vec<HashMap<std::string::String, AttributeValue>> =
            scan_output.items().into_iter().cloned().collect();

        let describe_table_response =
            describe_table(profile.clone().unwrap_or("default".to_string()), table_name).await?;
        print_dynamo_items(&samples, &describe_table_response);

        Ok(samples)
    }

    pub async fn scan_table(
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

        let config = get_aws_client_config(&profile.clone().unwrap_or("default".to_string())).await;
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

        let describe_table_response =
            describe_table(profile.clone().unwrap_or("default".to_string()), table_name).await?;
        print_dynamo_items(&all_items, &describe_table_response);
        Ok(all_items)
    }

    pub async fn empty_table(&self, profile: Option<String>) -> Result<(), CustomError> {
        let DynamoDBAction::EmptyTable { table_name } = self else {
            return Err(CustomError::UnexpectedActionVariant(
                "Unexpected action for empty_table".to_string(),
            ));
        };

        let config = get_aws_client_config(&profile.clone().unwrap_or("default".to_string())).await;
        let db_client = DynamoClient::new(&config);

        let describe_table_response =
            describe_table(profile.clone().unwrap_or("default".to_string()), table_name).await?;

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

    pub async fn query(&self, profile: Option<String>) -> Result<(), CustomError> {
        let DynamoDBAction::Query { table_name, pk, sk } = self else {
            return Err(CustomError::UnexpectedActionVariant(
                "Unexpected action for query".to_string(),
            ));
        };

        if pk.is_empty() {
            return Err(CustomError::InvalidInput(
                "Partition key (pk) must be provided for query.".to_string(),
            ));
        }

        let config = get_aws_client_config(&profile.unwrap_or("default".to_string())).await;
        let db_client = DynamoClient::new(&config);

        Ok(())
    }
}
