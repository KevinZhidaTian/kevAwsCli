use crate::cli::get_aws_client_config;
use crate::dynamodb::actions::DynamoDBAction;
use crate::dynamodb::formatter::to_attribute_value;
use crate::dynamodb::utils::{describe_table, get_key_name, print_dynamo_items};
use crate::error::CustomError;
use aws_sdk_dynamodb::Client as DynamoClient;
use aws_sdk_dynamodb::operation::describe_table::DescribeTableOutput;
use aws_sdk_dynamodb::types::{KeyType, ScalarAttributeType};

impl DynamoDBAction {
    pub async fn query(&self, profile: &String) -> Result<(), CustomError> {
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

        let config = get_aws_client_config(profile).await;
        let db_client = DynamoClient::new(&config);

        let table_desc = describe_table(profile, table_name).await?;
        let pk_name = match get_key_name(&table_desc, KeyType::Hash) {
            Some(name) => Some(name),
            None => {
                return Err(CustomError::InvalidInput(
                    "Partition key name could not be determined from table schema.".to_string(),
                ));
            }
        };
        let sk_name = match get_key_name(&table_desc, KeyType::Range){
            Some(name) => Some(name),
            None => {
                if sk.is_some() {
                    return Err(CustomError::InvalidInput(
                        "Sort key (sk) provided but table does not have a sort key.".to_string(),
                    ));
                } else {
                    None
                }
            },
        };

        fn get_attribute_type<'a>(
            table_desc: &'a DescribeTableOutput,
            attribute_name: &str,
        ) -> Option<&'a ScalarAttributeType> {
            table_desc
                .table()
                .and_then(|t| {
                    t.attribute_definitions()
                        .iter()
                        .find(|def| def.attribute_name == attribute_name)
                })
                .map(|def| &def.attribute_type)
        }

        let pk_attribute_type = match pk_name {
            Some(ref name) => get_attribute_type(&table_desc, name.as_str()),
            None => {
                return Err(CustomError::InvalidInput(
                    "Partition key name could not be determined from table schema.".to_string(),
                ));
            }
        };

        let sk_attribute_type = match sk_name {
            Some(ref name) => get_attribute_type(&table_desc, name.as_str()),
            None => {
                if sk.is_some() {
                    return Err(CustomError::InvalidInput(
                        "Sort key (sk) provided but table does not have a sort key.".to_string(),
                    ));
                } else {
                    None
                }
            },
        };

        let key_condition = if sk_attribute_type.is_some() {
            "#pk = :pk_val AND #sk = :sk_val"
        } else {
            "#pk = :pk_val"
        };

        let query_request = if let Some(pk_type) = pk_attribute_type {
            db_client
            .query()
            .table_name(table_name)
            .key_condition_expression(key_condition)
            .expression_attribute_names("#pk", pk_name.as_ref().unwrap())
            .expression_attribute_values(
                ":pk_val",
                to_attribute_value(pk, pk_type),
            )
        } else {
            return Err(CustomError::InvalidInput(
                "Partition key name could not be determined from table schema.".to_string(),
            ));
        };

        let query_request = if let Some(sk_type) = sk_attribute_type {
            query_request
                .expression_attribute_names("#sk", sk_name.as_ref().unwrap())
                .expression_attribute_values(
                    ":sk_val",
                    to_attribute_value(sk.as_ref().unwrap(), sk_type),
                )
        } else if let Some(sk_val) = sk {
            return Err(CustomError::InvalidInput(format!(
                "Sort key (sk) provided but table does not have a sort key: {}",
                sk_val
            )));
        } else {
            query_request
        };

        let result = query_request.send().await;

        match result {
            Ok(output) => {
                let items = output.items();
                print_dynamo_items(&items, &table_desc);
            }
            Err(e) => {
                return Err(CustomError::DynamoError(e.into()));
            }
        }
        Ok(())
    }
}
