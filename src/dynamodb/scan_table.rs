use crate::cli::get_aws_client_config;
use crate::dynamodb::actions::DynamoDBAction;
use crate::dynamodb::utils::{describe_table, print_dynamo_items};
use crate::error::CustomError;
use aws_sdk_dynamodb::{Client as DynamoClient, types::AttributeValue};
use std::collections::HashMap;

impl DynamoDBAction {
    pub async fn scan_table(
        &self,
        profile: &String,
    ) -> Result<Vec<HashMap<String, AttributeValue>>, CustomError> {
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

        let config = get_aws_client_config(profile).await;
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

        let describe_table_response = describe_table(profile, table_name).await?;
        print_dynamo_items(&all_items, &describe_table_response);
        Ok(all_items)
    }
}
