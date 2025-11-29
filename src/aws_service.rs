use crate::dynamodb::actions::DynamoDBAction;
use crate::{cli::AWSService, error::CustomError};
impl AWSService {
    pub async fn execute(&self, profile: Option<String>) -> Result<(), CustomError> {
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
                DynamoDBAction::Query { .. } => {
                    action.query(profile).await?;
                }
            },
            AWSService::S3 { bucket_name } => {
                println!("S3 Bucket Name: {}", bucket_name);
            }
        }
        Ok(())
    }
}
