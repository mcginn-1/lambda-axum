
#![allow(unused)]

use std::collections::HashMap;
use aws_sdk_dynamodb::types::{AttributeValue, ReturnValue};
use aws_sdk_dynamodb::{Client, Error};
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::get_item::{GetItemError, GetItemOutput};
use aws_smithy_types::error::metadata::ProvideErrorMetadata;
use axum::Json;
use base64::Engine;
use serde::{de, Deserialize, Serialize};
use serde_dynamo::aws_sdk_dynamodb_1::{from_items, to_item};
use serde_json::json;
use base64::{decode};
use serde_json::from_str;
use anyhow::{anyhow, Context, Result};


use base64::{encode};
use crate::dynamo::DynamoError;
use crate::dynamo::DynamoError::DynError;



fn handle_error<E>(e: E) -> DynamoError
where
    E: std::fmt::Display,
{
    DynamoError::DynErrorExp {exp: e.to_string()}
}

/// Coverts a Base64 key sent via the REST query into a
/// type and then into a HashMap<String, AttributeValue>
/// readable by DynamoDB
pub fn get_last_evaluated_key<T>(paginator_token: &str)
    -> Result<HashMap<String, AttributeValue>, anyhow::Error>
where
    T: serde::de::DeserializeOwned + Serialize
{
    let json_key = decode_base64_to_json(paginator_token)?;
    // Get UserTableKey
    let last_key_struct_from_json =
        serde_json::from_str::<T>(json_key.as_str())?;
    // Convert key to the Dynamo HashTable<String, AttributeValue> format
    let last_evaluated_key =
        serde_dynamo::to_item(last_key_struct_from_json)?;
    Ok(last_evaluated_key)
}

/// Creates a Base64 version of the last evaluated key from dynamo HashMap<String, AttributeValue>
pub fn generate_evaluated_key_base64<T> (last_evaluated_key: HashMap<String, AttributeValue>)
                                                -> Result<String, anyhow::Error>
   where T: Serialize + serde::de::DeserializeOwned
{
    let last_table_key: T = serde_dynamo::aws_sdk_dynamodb_1::from_item(last_evaluated_key)?;
    let last_evaluated_key_json = json!(last_table_key).to_string();
    let last_evaluated_key_base64 = encode(&last_evaluated_key_json);
    Ok(last_evaluated_key_base64)
}

// Only used in UserTable
pub async fn create_entity_serde<T: serde::Serialize>(client: &Client, entity: T, table: &String)
                                                      -> Result<(), anyhow::Error> {
    // Create dynamoDb HashMap<String, AttributeValue> entity
    let entity = serde_dynamo::aws_sdk_dynamodb_1::to_item(entity)?;

    // Write to dynamo
    client
        .put_item()
        .table_name(table)
        .set_item(Some(entity))
        .send()
        .await
        .map_err(|e| {
            e.into_service_error()
        })?;

    Ok(())
}


/// DynamoDB - Delete Item by Key
///
///
pub async fn delete_by_key_attribute_value_serde(
    client: &Client,
    table_name: &str,
    // attribute_name: &str,
    // attribute_value_str: &str,
    key: HashMap<String, AttributeValue>
) -> Result<(), DynamoError> {

    let result =
        client
            .delete_item()
            .table_name(table_name)
            .set_key(Some(key))
            .send()
            .await
            .map_err(|e| DynamoError::DynErrorExp {exp: e.into_service_error().to_string()})?
        ;
    Ok(())
}


pub fn decode_base64_to_json(token: &str) -> Result<String, base64::DecodeError> {
    // Decode the base64 token
    let decoded_bytes = base64::prelude::BASE64_URL_SAFE.decode(token)?;
    // Convert Vec<u8> to String
    let json_string = String::from_utf8(decoded_bytes)
        .map_err(|err| base64::DecodeError::InvalidLength)?;
    Ok(json_string.to_string())
}

/// DynamoDB - Query Item by Key
///
/// Rust Generics with Deserialize
///     https://stackoverflow.com/questions/54851996/rust-and-serde-deserializing-using-generics
///
pub async fn query_items_key_attribute_value_serde<T>(
    client: &Client,
    table_name: &str,
    key: HashMap<String, AttributeValue>
) -> Result<Option<T>, anyhow::Error>
where
    T: de::DeserializeOwned
{
    let result =
        client
            .get_item()
            .table_name(table_name)
            .set_key(Some(key))
            .send()
            .await
            .map_err(|e| e.into_service_error()).context("get item issue")?
        ;

    match result.item {
        Some(attribute_hash_map) => {
            // let i = serde_dynamo::aws_sdk_dynamodb_1::from_item(result.item.unwrap())
            let i = serde_dynamo::aws_sdk_dynamodb_1::from_item(attribute_hash_map).context("from item issue")?;
                // .map_err(|e| DynamoError::DynErrorExp {exp: e.to_string()})?;
            Ok(i)

        }
        None => {Ok(None)}
    }
}

