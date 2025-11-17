use aws_sdk_dynamodb::types::AttributeValue;

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
