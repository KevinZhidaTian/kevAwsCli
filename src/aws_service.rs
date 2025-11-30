use crate::dynamodb::actions::DynamoDBAction;
use crate::{cli::AWSService, error::CustomError};
impl AWSService {
    pub async fn execute(&self, profile: Option<String>) -> Result<(), CustomError> {
        let profile_str = &profile.unwrap_or("default".to_string());
        match self {
            AWSService::DynamoDB { action } => match action {
                DynamoDBAction::ListSamples { .. } => {
                    action.list_samples(profile_str).await?;
                }
                DynamoDBAction::EmptyTable { .. } => {
                    action.empty_table(profile_str).await?;
                }
                DynamoDBAction::ScanTable { .. } => {
                    action.scan_table(profile_str).await?;
                }
                DynamoDBAction::Query { .. } => {
                    action.query(profile_str).await?;
                }
            },
            AWSService::S3 { bucket_name } => {
                println!("S3 Bucket Name: {}", bucket_name);
            }
        }
        Ok(())
    }
}
