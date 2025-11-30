use crate::cli::get_aws_client_config;
use crate::dynamodb::actions::DynamoDBAction;
use crate::dynamodb::utils::{describe_table, print_dynamo_items};
use crate::error::CustomError;
use aws_sdk_dynamodb::{
    Client as DynamoClient, error::SdkError, operation::scan::ScanError, types::AttributeValue,
};
use std::collections::HashMap;

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
}
