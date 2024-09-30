
#![allow(unused)]
pub mod user;
mod auth;
mod jwk;
pub mod dynamo;
mod dynamo_query_helpers;
mod error;
mod modyne;
mod item;
mod user_table;
mod user_table_handlers;
mod item_handlers;

use crate::item_handlers::*;
use crate::user::create_user;
use crate::user_table_handlers::*;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::{
    extract::Path,
    response::Json,
    routing::{get, post},
    Router,
};
use lambda_http::tracing::log::info;
use lambda_http::{run, tracing, Error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::env::set_var;
use std::net::SocketAddr;

use crate::auth::{AuthError, CurrentUser};
// use crate::dynamo::{dynamo_add_item_rest, dynamo_call, query_items_by_key_username_rest, query_items_by_field_rest, query_items_by_scan_serde_rest, dynamo_add_item_rest_serde, delete_items_by_key_username_rest, query_accountusers_handler, query_account_users_by_date_range_handler, create_user_table_serde_rest_handler, delete_user_table_serde_rest_handler, update_user_table_serde_rest_handler};
use crate::dynamo_query_helpers::query_items_key_attribute_value_serde;
// use crate::dynamo::dynamo;
use crate::jwk::FBTokenClaims;
use axum::http::Request;
use axum::middleware::Next;
use axum::routing::{delete, put};
use axum::{body::Body, middleware, response::{IntoResponse, Response}, Extension};
use jsonwebtoken::TokenData;


#[derive(Debug, thiserror::Error)]
pub enum SystemError {
    #[error("{0}")]
    AuthError(#[from] AuthError),
}

// Run Locally: cargo lambda watch --invoke-port 9003
#[tokio::main]
async fn main() -> Result<(), Error> {
    // Running axum as an AWS cloud function
    // More examples can be found at:
    // https://github.com/awslabs/aws-lambda-rust-runtime/blob/main/examples/http-axum/src/main.rs
    // Use the following key to ignore test stage in path if using API Gatway
    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

    // required to enable CloudWatch error logging by the runtime
    tracing::init_default_subscriber();

    let app = Router::new()
        .route("/", get(root))

        // ******** DynamoDb Handlers ********
        //
        // See the Readme.txt file for loading a sample UserTable
        // into DynamoDb to run these commands and queries

        // Creates a UserTable Entity
        .route(
            "/create_user_table_entity",
            post(create_user_table_serde_rest_handler)
        )
        .route(
            "/update_user_table_entity",
            put(update_user_table_serde_rest_handler)
        )
        // Queries UserTable using key of User, manual
        .route(
            "/dynamo_query_serde_by_key_user_table/:user/:order",
            get(query_items_by_key_account_user_rest),
        )
        .route(
            "/delete_user_table_entity/:user_id/:order_id",
            delete(delete_user_table_serde_rest_handler)
        )
        // Queries for User_Table
        // curl -H "Content-Type: application/json" \
        // -X GET "http://localhost:{{port}}/dynamo_query_accountusers_handler?page_size=2&token=RETURNED_TOKEN_FROM_APP-TOKEN_IN_HEADER"
        .route(
            "/dynamo_query_accountusers_handler",
            get(query_accountusers_handler)
            // get(query_accountusers_handler).layer(middleware::from_fn(get_paginator_token))
        )
        .route(
            // Format start_date=2025-07-10T19:00:22.819Z end_date=2025-07-10T19:00:22.819Z
            "/dynamo_query_account_users_by_date_range",
            get(query_account_users_by_date_range_handler)
            // get(query_accountusers_handler).layer(middleware::from_fn(get_paginator_token))
        )


        // ****** FIREBASE AUTH JWT Example ******
        //
        // get_fb_token_claims will return aud, sub and iss from Firebase Token,
        // {
        //     "aud": "{project id}",
        //     "sub": "{user id}",
        //     "iss": "https://securetoken.google.com/{project id}"
        // }
        .route(
            "/get_fb_token_claims",
            get(get_fb_token_claims)
                .layer(middleware::from_fn(auth::authorize_firebase)),
        )
        // .layer(middleware::from_fn(logging_middleware))


        // ******** Modyne Handlers ********
        // The following handlers use the Modyne library to
        //  interact with DynamoDB.

        // Creates a session, returns a session_id,
        // which can be used to GET the session
        .route(
            "/create_session_modyne",
            post(crate::modyne::create_session_modyne_handler)
        )
        .route(
            "/get_session_modyne/:session_id",
            get(crate::modyne::get_session_modyne_handler)
        )
        .route(
            "/update_session_modyne/:session_id/:username",
            put(crate::modyne::update_session_username_modyne_handler)
        )
        .route(
            "/delete_session_modyne/:session_id",
            delete(crate::modyne::delete_session_modyne_handler)
        )

        // End Modyne Handlers


        .route(
            "/dynamo",
            get(dynamo_call),
        )
        .route(
            "/dynamo_add",
            post(dynamo_add_item_rest_serde),
            // get(dynamo_add_item_rest),
        )
        .route(
            "/dynamo_query_items_by_field_rest",
            get(query_items_by_field_rest),
        )
        .route(
            "/dynamo_query_items_by_scan_serde_rest",
            get(query_items_by_scan_serde_rest),
        )
        .route(
            "/dynamo_query_serde_by_key_username/:username",
            get(query_items_by_key_username_rest),
        )
        .route(
            "/dynamo_delete_serde_by_key_attribute_value/:username",
            delete(delete_items_by_key_username_rest),
        )


        //  The following is an implementation of the example
        //  from the following LogRocket tutorial
        //  https://blog.logrocket.com/using-rust-axum-build-jwt-authentication-api/
        //  Shows an example of creating a user, signing in and receiving a custom
        //  JWT in response, then using that token to authorize on the 'hello' function

        // Example to create a user.
        // curl --header "Content-Type: application/json" \
        // --request POST \
        // --data '{"username":"Mike"}' \
        // http://localhost:9003/create_user
        .route("/create_user", post(create_user))
        // Returns a token after signing in
        // with SignInData in POST. Hardcoded user with
        // Email: email: "myemail@gmail.com", password: "okon"
        .route("/signin", post(auth::sign_in))
        // The authorize middleware is getting the current user from
        //  the token and calling the hello function and placing
        //  the user in the function call as an extension parameter
        .route(
            "/get_user_custom_token",
            get(hello).layer(middleware::from_fn(auth::authorize)),
        )


        // Examples from the aws lambda axum code
        // https://github.com/awslabs/aws-lambda-rust-runtime/blob/main/examples/http-axum/src/main.rs
        .route("/foo", get(get_foo).post(post_foo))
        .route("/foo/:name", post(post_foo_name))
        .route("/parameters", get(get_parameters))
        .route("/health/", get(health_check))

        ;

    run(app).await



    // To run locally on an axum server and deploy using Docker,
    // uncomment the following code and add axum_server dependency
    // You can then test locally and deploy as Docker image.
    //
    // // If app can't find an AWS LAMBDA Environment variable,
    // // Then switch to using a local server.
    // match env::var("AWS_LAMBDA_FUNCTION_NAME") {
    //     Ok(_) => {
    //         // To run just on Lambda
    //         run(app).await
    //     }
    //     Err(err) => {
    //         // Run app in locally if not on lambda
    //         info!( "No env var for lambda: {}, running locally." ,err);
    //         let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    //         info!("listening on {}", addr);
    //         axum_server::bind(addr)
    //             .serve(app.into_make_service())
    //             .await
    //             .unwrap();
    //         Ok(())
    //
    //     }
    // }
}


async fn root() -> Json<Value> {
    Json(json!({ "msg": "I am GET /" }))
}

async fn get_foo() -> Json<Value> {
    Json(json!({ "msg": "I am GET /foo" }))
}

async fn post_foo() -> Json<Value> {
    Json(json!({ "msg": "I am POST /foo" }))
}

async fn post_foo_name(Path(name): Path<String>) -> Json<Value> {
    Json(json!({ "msg": format!("I am POST /foo/:name, name={name}") }))
}

#[derive(Deserialize, Serialize)]
struct Params {
    first: Option<String>,
    second: Option<String>,
}
async fn get_parameters(Query(params): Query<Params>) -> Json<Value> {
    Json(json!({ "request parameters": params }))
}



// These are from the axum JWT tutorial
// https://blog.logrocket.com/using-rust-axum-build-jwt-authentication-api/
#[derive(Serialize, Deserialize)]
struct UserResponse {
    email: String,
    first_name: String,
    last_name: String
}

// Extensions are used to pass local state
// The authorize function places the currentUser in the extension
// and moves this from the middleware result to the hello function parameter.
// pub async fn hello(Extension(currentUser): Extension<CurrentUser>) -> impl IntoResponse {
pub async fn hello(Extension(currentUser): Extension<CurrentUser>) -> impl IntoResponse {
    Json(UserResponse {
        email: currentUser.email,
        first_name: currentUser.first_name,
        last_name: currentUser.last_name
    })
}
#[derive(Serialize, Deserialize)]
struct ClaimsResponse {
    aud: String,
    sub: String,
    iss: String
}

// Extensions are used to pass local state
// and moves this from the middleware result to the hello function parameter.
// pub async fn hello(Extension(currentUser): Extension<CurrentUser>) -> impl IntoResponse {
pub async fn get_fb_token_claims(Extension(token_claims): Extension<TokenData<FBTokenClaims>>) -> impl IntoResponse {
    Json(ClaimsResponse {
        aud: token_claims.claims.aud,
        sub: token_claims.claims.sub,
        iss: token_claims.claims.iss,
    })
}

// Alternative boilerplate to have a create_user
#[derive(Deserialize)]
struct CreateUser {
    email: String,
    password: String,
}
async fn create_user_2 (payload: Option<Json<Value>>) {
    if let Some(payload) = payload {
        // We got a valid JSON payload
    } else {
        // Payload wasn't valid JSON
    }
}

/// Example on how to return status codes and data from an Axum function
async fn health_check() -> (StatusCode, String) {
    let health = true;
    match health {
        true => (StatusCode::OK, "Healthy!".to_string()),
        false => (StatusCode::INTERNAL_SERVER_ERROR, "Not healthy!".to_string()),
    }
}

async fn logging_middleware(req: Request<Body>, next: Next) -> Response {
    println!("Received a request to {}", req.uri());
    next.run(req).await
}


