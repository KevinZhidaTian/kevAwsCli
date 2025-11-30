use crate::cli::get_aws_client_config;
use crate::dynamodb::actions::DynamoDBAction;
use crate::dynamodb::utils::describe_table;
use crate::error::CustomError;
use aws_sdk_dynamodb::{Client as DynamoClient, types::KeyType};

impl DynamoDBAction {
    pub async fn empty_table(&self, profile: &String) -> Result<(), CustomError> {
        let DynamoDBAction::EmptyTable { table_name } = self else {
            return Err(CustomError::UnexpectedActionVariant(
                "Unexpected action for empty_table".to_string(),
            ));
        };

        let config = get_aws_client_config(profile).await;
        let db_client = DynamoClient::new(&config);

        let describe_table_response = describe_table(profile, table_name).await?;

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

        let all_items = self.scan_table(&profile).await?;

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
