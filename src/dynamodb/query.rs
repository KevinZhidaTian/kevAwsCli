use crate::cli::get_aws_client_config;
use crate::dynamodb::actions::DynamoDBAction;
use crate::error::CustomError;
use aws_sdk_dynamodb::Client as DynamoClient;

impl DynamoDBAction {
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
