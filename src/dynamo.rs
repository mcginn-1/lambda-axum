

#![allow(unused)]

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
use crate::dynamo_query_helpers::*;
use crate::item::*;
use crate::{ UserResponse};
use crate::user_table::{query_by_date_range_serde_dynamo, query_by_sorted_dates_serde_dynamo, UpdateUserTable, UserTable};

#[derive(Debug, thiserror::Error)]
pub enum DynamoError {
    #[error("Error in Dynamo")]
    DynError ,
    #[error("Error in Dynamo: {exp:?}")]
    DynErrorExp {exp: String},
    // #[error("Error in Dynamo: {0}")]
    // DynErrorFrom {#[from] aws_smithy_runtime_api::client::result::SdkError},

}

// #[derive(Serialize, Deserialize)]
pub struct StatResp {
    pub result: String,
    pub message: String,
    pub status_code: StatusCode
}

impl StatResp {
    /// Creates a new `StatusResponse` with the provided parameters
    /// `result`: A result string
    /// `message`: A message string
    /// `status_code`: A status code identifier
    pub fn new(result: &str, message: &str, status_code: StatusCode) -> Self {
        let r = result.to_string();
        let m = message.to_string();
        StatResp {
            result: r,
            message: m,
            status_code,
        }
    }
}
#[derive(Serialize)]
struct StatusResponseOut {
    result: String,
    message: String,
}
impl IntoResponse for StatResp {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        let body = Json(
            StatusResponseOut{ result: self.result, message: self.message }
        );
        (self.status_code, body).into_response()
    }
}


