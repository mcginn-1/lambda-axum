use anyhow::Context;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::types::AttributeValue;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserTable {
    // #[partition]
    pub UserId: String,
    // #[range]
    pub OrderId: String,
    pub product: String,
    pub price: f64,
    pub gsi_pk: i64,
    pub date_ordered: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateUserTable {
    pub UserId: String,
    pub OrderId: String,
    pub product: String,
    pub price: f64,
}

impl UserTable {

}


/// Last key returned in Dynamo paginated query
/// Convert to this then JSON Base64 to return
/// on HTTP header
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserTableKey {
    pub UserId: String,
    pub OrderId: String,
    pub gsi_pk: i64,
    pub date_ordered: String,
}

#[derive(Clone, Debug)]
pub struct PaginatedOutput<T> {
    pub key: Option<String>,
    pub output: T,
}


/// Serde dynamo - https://docs.rs/serde_dynamo/latest/serde_dynamo/aws_sdk_dynamodb_1/index.html
///
///  aws dynamodb query \
// --table-name UserTable \
// --index-name gsi1 \
// --key-condition-expression "#gsi1_pk = :gsi1_pk_val" \
// --expression-attribute-values '{":gsi1_pk_val":{"N":"1"}}' \
// --expression-attribute-names '{"#gsi1_pk":"gsi_pk"}' \
// --no-scan-index-forward \
// --max-items 2
///
pub async fn query_by_sorted_dates_serde_dynamo(
    client: &Client,
    table_name: &str,
    page_size: Option<i32>,
    paginator_token: Option<&String>
) -> anyhow::Result<PaginatedOutput<Vec<UserTable>>, anyhow::Error> {

    let mut query = client
        .query()
        .table_name(table_name)
        .index_name("gsi1")
        .scan_index_forward(false)
        .key_condition_expression("#gsi1_pk = :gsi1_pk_val")
        .expression_attribute_values(":gsi1_pk_val", AttributeValue::N("1".to_string()))
        .expression_attribute_names("#gsi1_pk","gsi_pk")
        ;

    // If there is a page_size parameter, add to query
    if let Some(limit) = page_size {
        query = query.clone().limit(limit);
    };

    // If there is a paginator_token start point parameter, add to query
    if let Some(paginator_token) = paginator_token {
        let last_evaluated_key =
            crate::dynamo_query_helpers::get_last_evaluated_key::<UserTableKey>(paginator_token.as_str())?;
            // crate::dynamo_add::UserTableKey::<UserTableKey>(paginator_token.as_str())?;


        query = query.clone().set_exclusive_start_key(Some(last_evaluated_key));
    }

    // Execute Query
    let results = query
        .send()
        .await
        .map_err(|e| {
            e.into_service_error()
        })?;

    // Handle Results
    if let Some(items) = results.items {
        let items = items.to_vec();
        // Convert from HashMap of AttributeValues to struct
        let users: Vec<UserTable> =
            serde_dynamo::aws_sdk_dynamodb_1::from_items(items)?;

        let last_evaluated_key_base64 =
            results.last_evaluated_key
                .map_or(None, | last_evaluated_key| {
                    Some(crate::dynamo_query_helpers::generate_evaluated_key_base64::<UserTableKey>(last_evaluated_key))
                }).transpose()?;

        Ok(PaginatedOutput{
            key: last_evaluated_key_base64,
            output: users,
        })
    } else {
        Ok(PaginatedOutput{
            key: None,
            output: vec![],
        })
    }
}

/// Serde dynamo - https://docs.rs/serde_dynamo/latest/serde_dynamo/aws_sdk_dynamodb_1/index.html
///
/// aws dynamodb query \
/// --table-name UserTable \
/// --index-name gsi1 \
/// --key-condition-expression "#gsi1_pk = :gsi1_pk_val AND #SK between :from AND :to" \
/// --expression-attribute-values '{":gsi1_pk_val":{"N":"1"}, ":from":{"S":"2023-10-03"}, ":to":{"S":"2026-10-07"}}' \
/// --expression-attribute-names '{"#gsi1_pk":"gsi_pk", "#SK":"date_ordered"}'
/// --no-scan-index-forward \
/// --max-items 2
///
///
///
pub async fn query_by_date_range_serde_dynamo(
    client: &Client,
    table_name: &str,
    page_size: Option<i32>,
    paginator_token: Option<&String>,
    start_date: String,
    end_date: String,
) -> Result<PaginatedOutput<Vec<UserTable>>, anyhow::Error> {

    let mut query = client
        .query()
        .table_name(table_name)
        .index_name("gsi1")
        .scan_index_forward(false)
        .key_condition_expression("#gsi1_pk = :gsi1_pk_val AND #SK between :from AND :to")
        .expression_attribute_values(":gsi1_pk_val", AttributeValue::N("1".to_string()),)
        .expression_attribute_values(":from", AttributeValue::S(start_date))
        .expression_attribute_values(":to", AttributeValue::S(end_date))
        // .expression_attribute_values(":from", AttributeValue::S("2025-07-10T19:00:22.819Z".to_string()),)
        // .expression_attribute_values(":to", AttributeValue::S("2026-07-10T19:00:22.819Z".to_string()),)
        // .expression_attribute_values(":from", AttributeValue::S("2024-10-03".to_string()),)
        // .expression_attribute_values(":to", AttributeValue::S("2025-10-07".to_string()),)
        .expression_attribute_names("#gsi1_pk","gsi_pk")
        .expression_attribute_names("#SK","date_ordered")
        ;

    // If there is a page_size parameter, add to query
    if let Some(limit) = page_size {
        query = query.clone().limit(limit);
    };

    // If there is a paginator_token start point parameter, add to query
    if let Some(paginator_token) = paginator_token {
        let last_evaluated_key =
            crate::dynamo_query_helpers::get_last_evaluated_key::<UserTableKey>(paginator_token.as_str())
                .context("Token Error")?;

        query = query.clone().set_exclusive_start_key(Some(last_evaluated_key));
    }

    // Execute Query
    let results = query
        .send()
        .await
        .map_err(|e| {
            let se = e.into_service_error();
            // handle_error(se)
            se
        })?;

    // Handle Results
    if let Some(items) = results.items {
        let items = items.to_vec();
        // Convert from HashMap of AttributeValues to struct
        let users: Vec<UserTable> =
            serde_dynamo::aws_sdk_dynamodb_1::from_items(items)?;
        // .map_err(handle_error)?;

        let last_evaluated_key_base64 =
            results.last_evaluated_key
                .map_or(None, | last_evaluated_key| {
                    Some(crate::dynamo_query_helpers::generate_evaluated_key_base64::<UserTableKey>(last_evaluated_key))
                }).transpose()?;

        Ok(PaginatedOutput{
            key: last_evaluated_key_base64,
            output: users,
        })
    } else {
        Ok(PaginatedOutput{
            key: None,
            output: vec![],
        })
    }
}
