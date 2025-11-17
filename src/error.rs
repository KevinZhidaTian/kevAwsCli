use aws_sdk_dynamodb::Error as DynamoError;
use std::fmt;

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
