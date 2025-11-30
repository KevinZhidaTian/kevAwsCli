use crate::cli::get_aws_client_config;
use crate::dynamodb::formatter::format_attribute_value;
use crate::error::CustomError;
use aws_sdk_dynamodb::Client as DynamoClient;
use aws_sdk_dynamodb::{
    operation::describe_table::DescribeTableOutput,
    types::{AttributeValue, KeyType},
};
use comfy_table::{Cell, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};
use std::collections::{BTreeSet, HashMap};

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
            let value = v.into();

            self.headers.insert(key.clone());

            if &key == &self.pk {
                row.insert(format!("{} (PK)", &self.pk), value);
            } else if Some(&key) == self.sk.as_ref() {
                row.insert(format!("{} (SK)", self.sk.as_ref().unwrap()), value);
            } else {
                row.insert(key, value);
            }
        }

        self.rows.push(row);
    }

    fn ordered_header(&self) -> Vec<String> {
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
            ordered_header.push(format!("{} (PK)", &self.pk));
        }

        if self
            .headers
            .contains(self.sk.as_ref().map_or("Null", |sk| sk))
        {
            ordered_header.push(format!("{} (SK)", self.sk.clone().unwrap()));
        }
        ordered_header.extend(other_headers);

        ordered_header
    }

    pub fn convert_to_table(&mut self) -> Table {
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

pub fn get_key_name(table_desc: &DescribeTableOutput, key_type: KeyType) -> Option<String> {
    table_desc
        .table
        .as_ref()
        .and_then(|t| t.key_schema.as_ref())
        .and_then(|ks| {
            ks.iter()
                .find(|schema| schema.key_type == key_type)
                .map(|schema| schema.attribute_name.clone())
        })
}

pub fn print_dynamo_items(
    items: &[HashMap<String, AttributeValue>],
    table_desc: &DescribeTableOutput,
) {
    if items.is_empty() {
        println!("No items found.");
        return;
    }

    let pk = get_key_name(table_desc, KeyType::Hash);
    let sk = get_key_name(table_desc, KeyType::Range);

    let mut dynamic_table = DynamicTable::new(pk.unwrap_or("PK".to_string()), sk);

    for item in items {
        let mut row = HashMap::new();
        for (key, value) in item {
            let cell_text = format_attribute_value(value);
            row.insert(key, cell_text);
        }

        dynamic_table.add_row(row);
    }

    let print_table = dynamic_table.convert_to_table();

    println!("{print_table}");
}

pub async fn describe_table(
    profile: &String,
    table_name: &str,
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
