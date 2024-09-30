use crate::dynamo::{DynamoError, StatResp};
use crate::item::*;

use std::collections::HashMap;
use anyhow::Context;
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::operation::create_table::CreateTableOutput;
use aws_sdk_dynamodb::types::{AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ProvisionedThroughput, ScalarAttributeType};
use axum::{Extension, Json};
use axum::extract::Query;
use axum::http::{HeaderValue, StatusCode};
use axum::http::header::ToStrError;
use axum::middleware::Next;
use axum::response::IntoResponse;
use chrono::Utc;
use jsonwebtoken::Header;
use lambda_http::{run, service_fn, tracing, Body, Error, Request, Response};
use lambda_http::tower::BoxError;
use lambda_runtime::IntoFunctionResponse;
use lambda_runtime_api_client::tracing::subscriber::fmt::format::json;
use serde::{Deserialize, Serialize};
use serde_dynamo::to_attribute_value;
use serde_json::json;
use crate::auth::{AuthError, CurrentUser, VerificationError};
use crate::dynamo::DynamoError::DynError;
// use crate::dynamo_add::{add_item, add_item_serde, Item, ItemOut, query_items_by_username, query_items_by_field_attribute_serde, query_items_key_attribute_value_serde, delete_by_key_attribute_value_serde, UserTable, query_by_date_range_serde_dynamo, query_by_sorted_dates_serde_dynamo, UpdateUserTable};
use crate::dynamo_query_helpers::*;
use crate::item::*;
use crate::{ UserResponse};
use crate::user_table::{query_by_date_range_serde_dynamo, query_by_sorted_dates_serde_dynamo, UpdateUserTable, UserTable};



pub async fn query_items_by_scan_serde_rest() -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);

    let table = "lambda_dynamo_2".to_string();

    match crate::item::query_items_scan_serde(&client, &table, "user1").await {
        Ok(items) => {
            axum::Json(items).into_response()

        }
        Err(e) => {
            axum::response::IntoResponse::into_response(
                StatResp {
                    result: "failure".to_string(),
                    message: e.to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR

                }
            )

        }
    }
}

pub async fn query_items_by_field_rest() -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);

    let table = "lambda_dynamo_2".to_string();

    match query_items_by_field_attribute_serde(&client, &table, "user1").await {
        Ok(items) => {
            axum::Json(items).into_response()

        }
        Err(e) => {
            axum::response::IntoResponse::into_response(
                StatResp {
                    result: "failure".to_string(),
                    message: e.to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR

                }
            )
        }
    }
}

pub async fn query_items_by_key_username_rest(
    axum::extract::Path(username): axum::extract::Path<String>
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let table = "lambda_dynamo_2".to_string();

    // Create the unique key of the record in DynamoDB in a way rusoto understands
    let key =
        HashMap::from([
            // Map of [ key_field_name, key_field_value_as_attribute_value ]
            (String::from("username"), serde_dynamo::to_attribute_value(username).unwrap()),
        ]);

    // match query_items_key_attribute_value_serde::<Item>(&client, &table, "username".to_string(),username.as_str()).await {
    match query_items_key_attribute_value_serde::<Item>(&client, &table, key).await {
        Ok(item) => match item {
            Some(item_out) => {axum::Json(item_out).into_response()}
            None => StatResp::new("failure", "no item found", StatusCode::OK).into_response()
        }
        Err(e) => StatResp::new("failure", e.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }
}


pub async fn dynamo_add_item_rest() -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);

    let item: Item = Item{
        // p_type: "123".to_string(),
        account_type: "123".to_string(),
        age: "23".to_string(),
        username: "user5".to_string(),
        first_name: "john".to_string(),
        last_name: "jones".to_string(),
    };
    let table = "lambda_dynamo_2".to_string();


    return match add_item(&client, item, &table).await {
        Ok(item_out) => {
            StatResp {
                result: "success".to_string(),
                message: "added item".to_string(),
                status_code: StatusCode::OK

            }
        }
        Err(e) => {
            println!("Error adding item: {}", e.to_string());
            StatResp {
                result: "failure".to_string(),
                message: e.to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR

            }
        }
    };
}

pub async fn dynamo_add_item_rest_serde(axum::extract::Json(payload): axum::extract::Json<Item>) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let table = "lambda_dynamo_2".to_string();

    let item = payload;

    return match add_item_serde(&client, item, &table).await {
        Ok(_) => {
            axum::response::IntoResponse::into_response(
                StatResp {
                    result: "success".to_string(),
                    message: "added item".to_string(),
                    status_code: StatusCode::OK,
                }                                           )
        }
        Err(e) =>  {
            axum::response::IntoResponse::into_response(
                StatResp {
                    result: "failure".to_string(),
                    message: e.to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                }                                           )
        }
    };
}



pub async fn delete_items_by_key_username_rest(axum::extract::Path(username): axum::extract::Path<String>) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let table = "lambda_dynamo_2".to_string();

    let key =
        HashMap::from([
            // Map of [ key_field_name, key_field_value_as_attribute_value ]
            (String::from("username"), serde_dynamo::to_attribute_value(username)
                .map_err(|e|
                    // DynamoError::DynErrorExp {exp: e.to_string()}
                    StatResp::new("failure", e.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
                ).unwrap()
            ),
        ]);

    // match delete_by_key_attribute_value_serde(&client, &table, "username", username.as_str(), key).await {
    match delete_by_key_attribute_value_serde(&client, &table, key).await {
        Ok(item) =>
            { StatResp::new("success", "deleted item", StatusCode::OK).into_response() }
        Err(e) => StatResp::new("failure", e.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }
}

// pub async fn dynamo(Extension(currentUser): Extension<CurrentUser>) -> impl IntoResponse {
pub async fn dynamo_call() -> impl IntoResponse {

    //  The following code came from the main() function in the exapmles
    //  for setting up tracing, config and client.
    //  Config.load from env seems to be deprecated
    // https://github.com/awslabs/aws-lambda-rust-runtime/blob/main/examples/http-dynamodb/src/main.rs

    // required to enable CloudWatch error logging by the runtime (already in main)
    // tracing::init_default_subscriber();

    //Get config from environment.
    let config = aws_config::load_from_env().await;
    //Create the DynamoDB client.
    let client = Client::new(&config);


    // run(service_fn(|event: Request| async {
    // handle_request(&client, event).await
    // }))
    // .await

    match create_table(&client, "lambda_dynamo_2", "username").await {
        Ok(create_table_output) => {
            axum::response::IntoResponse::into_response(
                StatResp {
                    result: "success".to_string(),
                    message: "created table".to_string(),
                    status_code: StatusCode::OK

                })
        }
        Err(e) => {
            axum::response::IntoResponse::into_response(
                StatResp {
                    result: "failure".to_string(),
                    message: e.to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR

                })

        }
    };

    // Json(StatusResponse {
    //     result: "success".to_string(),
    //     message: "processed dynamodb transaction".to_string(),
    //     status_code: StatusCode::OK
    //
    // })

    // Json
    axum::response::IntoResponse::into_response
        (StatResp {
            result: "success".to_string(),
            message: "processed dynamodb transaction".to_string(),
            status_code: StatusCode::OK

        })
}

// Code is from AWS website
pub async fn create_table(
    client: &Client,
    table: &str,
    key: &str,
    // ) -> Result<CreateTableOutput, Error> {
) -> Result<CreateTableOutput, DynamoError>  {
    let a_name: String = key.into();
    let table_name: String = table.into();

    let ad = AttributeDefinition::builder()
        .attribute_name(&a_name)
        .attribute_type(ScalarAttributeType::S)
        .build()
        // .map_err(Err(Error))?;
        .map_err(|_| DynError)?;

    let ks = KeySchemaElement::builder()
        .attribute_name(&a_name)
        .key_type(KeyType::Hash)
        .build()
        // .map_err(Error::BuildError)?;
        .map_err(|_| DynError)?;

    let pt = ProvisionedThroughput::builder()
        .read_capacity_units(10)
        .write_capacity_units(5)
        .build()
        // .map_err(Error::BuildError)?;
        .map_err(|_| DynError)?;

    let create_table_response = client
        .create_table()
        .table_name(table_name)
        .key_schema(ks)
        .attribute_definitions(ad)
        .provisioned_throughput(pt)
        .send()
        .await;

    match create_table_response {
        Ok(out) => {
            println!("Added table {} with key {}", table, key);
            Ok(out)
        }
        Err(e) => {
            eprintln!("Got an error creating table:");
            eprintln!("{}", e);
            Err(DynError)
        }
    }
}


