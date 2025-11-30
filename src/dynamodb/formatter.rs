use aws_sdk_dynamodb::types::{AttributeValue, ScalarAttributeType};

pub fn format_attribute_value(value: &AttributeValue) -> String {
    match value {
        AttributeValue::S(s) => s.clone(),
        AttributeValue::N(n) => n.clone(),
        AttributeValue::Bool(b) => b.to_string(),
        AttributeValue::Null(_) => "NULL".to_string(),
        AttributeValue::B(b) => format!("<Binary: {} bytes>", b.as_ref().len()),
        AttributeValue::Ss(ss) => {
            format!("[{}]", ss.iter().cloned().collect::<Vec<_>>().join(", "))
        }
        AttributeValue::Ns(ns) => {
            format!("[{}]", ns.iter().cloned().collect::<Vec<_>>().join(", "))
        }
        AttributeValue::Bs(bs) => format!("<BinarySet: {} items>", bs.len()),
        AttributeValue::L(list) => {
            let items: Vec<String> = list.iter().map(format_attribute_value).collect();
            format!("[{}]", items.join(", "))
        }
        AttributeValue::M(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_attribute_value(v)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
        _ => format!("{:?}", value),
    }
}

pub fn to_attribute_value(raw_value: &str, attribute_type: &ScalarAttributeType) -> AttributeValue {
    match attribute_type {
        ScalarAttributeType::S => AttributeValue::S(raw_value.to_string()),
        ScalarAttributeType::N => AttributeValue::N(raw_value.to_string()),
        ScalarAttributeType::B => AttributeValue::B(raw_value.as_bytes().to_vec().into()),
        _ => AttributeValue::S(raw_value.to_string()), // Default to String if type is unknown
    }
}
