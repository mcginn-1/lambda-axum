use std::collections::HashMap;
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::Client;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use serde::Deserialize;
use crate::dynamo::StatResp;
use crate::dynamo_query_helpers::query_items_key_attribute_value_serde;
use crate::user_table::*;

pub async fn query_items_by_key_account_user_rest(
    axum::extract::Path((user, order)): axum::extract::Path<(String, String)>,
    // axum::extract::Path(order): axum::extract::Path<String>
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let table = "UserTable".to_string();

    // Create the unique key of the record in DynamoDB in a way rusoto understands
    let key =
        HashMap::from([
            // Map of [ key_field_name, key_field_value_as_attribute_value ]
            (String::from("UserId"), serde_dynamo::to_attribute_value("u#".to_string() + &*user).unwrap()),
            (String::from("OrderId"),  serde_dynamo::to_attribute_value("o#".to_string() + &*order).unwrap()),
        ]);

    match query_items_key_attribute_value_serde::<UserTable>
        // (&client, &table, "UserId".to_string(), user.as_str()).await {
        (&client, &table, key).await {
        Ok(item) => match item {
            Some(item_out) => {axum::Json(item_out).into_response()}
            None => StatResp::new("failure", "no item found", StatusCode::OK).into_response()
        }
        Err(e) => {
            // e.chain().skip(0).for_each(|cause| println!("because: {}", cause.to_string()));
            dbg!("{:?}", &e);
            StatResp::new("failure", e.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn query_items_by_key_account_user_dynamo_helper_rest(
    axum::extract::Path((user, order)): axum::extract::Path<(String, String)>,
    // axum::extract::Path(order): axum::extract::Path<String>
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let table = "UserTable".to_string();

    // Create the unique key of the record in DynamoDB in a way rusoto understands
    let key =
        HashMap::from([
            // Map of [ key_field_name, key_field_value_as_attribute_value ]
            (String::from("UserId"), serde_dynamo::to_attribute_value("u#".to_string() + &*user).unwrap()),
            (String::from("OrderId"),  serde_dynamo::to_attribute_value("o#".to_string() + &*order).unwrap()),
        ]);

    match query_items_key_attribute_value_serde::<UserTable>
        // (&client, &table, "UserId".to_string(), user.as_str()).await {
        (&client, &table, key).await {
        Ok(item) => match item {
            Some(item_out) => {axum::Json(item_out).into_response()}
            None => StatResp::new("failure", "no item found", StatusCode::OK).into_response()
        }
        Err(e) => {
            // e.chain().skip(0).for_each(|cause| println!("because: {}", cause.to_string()));
            dbg!("{:?}", &e);
            StatResp::new("failure", e.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}



// #[derive(DynamoDb)]
// struct OtherStruct {
//     #[partition]
//     id: String,
//     #[range]
//     range_id: String
//     // other values
// }




// #[derive(Deserialize)]
// pub struct Pagination {
//     token: String
//     // page: usize,
//     // per_page: usize,
// }

#[derive(Debug, Deserialize, Clone)]
pub struct Paginator {
    pub token: String,
    pub page_size: i32,
}

impl Default for Paginator {
    fn default() -> Self {
        Self {
            token: "".to_string(),
            page_size: 0,
        }
    }
}
//

pub async fn query_accountusers_handler(
    Query(params): Query<HashMap<String, String>>
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let table = "UserTable".to_string();


    let paginator_page_size_option: Option<i32> = match params.get("page_size") {
        Some(x) => {
            if let Ok(n) = x.parse::<i32>() {
                Some(n)
            } else {
                return StatResp::new("failure", "Invalid page size", StatusCode::BAD_REQUEST).into_response()
            }
        }
        None => {None}
    };

    // Get Option of Token &String. If it's empty (present but blank), set to None
    // It was already None if not present based on the HashMap get.
    let paginator_token_option: Option<&String> = params.get("token")
        .map_or(None, |token| {if token == "" {None} else {Some(token)}});

    match query_by_sorted_dates_serde_dynamo(
        &client,
        &table,
        paginator_page_size_option,
        paginator_token_option
    ).await {
        Ok(output) =>     {
            let item = output.output;
            let mut response = axum::Json(item).into_response();
            if let Some(token) = output.key {
                response.headers_mut().append("app-token", token.parse().unwrap());
            }
            response
        }
        Err(e) => StatResp::new("failure", e.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }
}

pub async fn query_account_users_by_date_range_handler(
    Query(params): Query<HashMap<String, String>>
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let table = "UserTable".to_string();

    let paginator_page_size_option: Option<i32> = match params.get("page_size") {
        Some(page_size_input) => {
            if let Ok(page_size) = page_size_input.parse::<i32>() {
                Some(page_size)
            } else {
                return StatResp::new("failure", "Invalid page size", StatusCode::BAD_REQUEST).into_response()
            }
        }
        None => {None}
    };

    // Get Option of Token &String. If it's empty (present but blank), set to None
    // It was already None if not present based on the HashMap get.
    let paginator_token_option: Option<&String> = params.get("token")
        .map_or(None, |token| {if token == "" {None} else {Some(token)}});

    let start_date: String = match params.get("start_date") {
        Some(start_date) => { start_date.to_string() }
        None => {return StatResp::new("failure", "missing start date parameter", StatusCode::BAD_REQUEST).into_response()}
    };

    let end_date: String = match params.get("end_date") {
        Some(start_date) => { start_date.to_string() }
        None => {return StatResp::new("failure", "missing end date parameter", StatusCode::BAD_REQUEST).into_response()}
    };


    match query_by_date_range_serde_dynamo(
        &client,
        &table,
        paginator_page_size_option,
        paginator_token_option,
        start_date,
        end_date,
    ).await {
        Ok(output) =>     {
            let item = output.output;
            let mut response = axum::Json(item).into_response();
            if let Some(token) = output.key {
                response.headers_mut().append("app-token", token.parse().unwrap());
            }
            response
        }
        // Err(e) => StatResp::new("failure", e.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
        Err(e) => {
                e.chain().skip(0).for_each(|cause| println!("because: {}", cause.to_string()));
                let e1 = e.to_string() + ": " + e.root_cause().to_string().as_str();
                StatResp::new("failure", e1.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
    }
}

pub async fn create_user_table_serde_rest_handler(
    axum::extract::Json(payload): axum::extract::Json<UpdateUserTable>
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let table = "UserTable".to_string();

    let update_user_table = payload;

    let mut user_table = UserTable{
        UserId: update_user_table.UserId,
        OrderId: update_user_table.OrderId,
        product: update_user_table.product,
        price: update_user_table.price,
        gsi_pk: 1,
        date_ordered: Utc::now().to_rfc3339().to_string(),
    };

    // let ut = UserTable{
    //     UserId: "u#user7".to_string(),
    //     OrderId: "o#order1".to_string(),
    //     product: "p#prod2".to_string(),
    //     price: 0.9118773868773133,
    //     gsi_pk: 1,
    //     date_ordered: "2024-09-08T02:37:08.733Z".to_string(),
    // };
    match crate::dynamo_query_helpers::create_entity_serde(&client, user_table, &table).await {
        Ok(item) =>
            { StatResp::new("success", "created item", StatusCode::OK).into_response() }
        Err(e) => StatResp::new("failure", e.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }
}

pub async fn update_user_table_serde_rest_handler(
    axum::extract::Json(payload): axum::extract::Json<UpdateUserTable>
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let table = "UserTable".to_string();

    let update_user_table = payload;

    // Create the unique key of the record in DynamoDB in a way rusoto understands
    let key =
        HashMap::from([
            // Map of [ key_field_name, key_field_value_as_attribute_value ]
            (String::from("UserId"), serde_dynamo::to_attribute_value(&*update_user_table.UserId).unwrap()),
            (String::from("OrderId"),  serde_dynamo::to_attribute_value(&*update_user_table.OrderId).unwrap()),
        ]);

    // Query to find the matching old UserTable
    let mut user_table = match query_items_key_attribute_value_serde::<UserTable>
        (&client, &table, key).await {
        Ok(item) => match item {
            Some(item_out) => item_out,
            None => return StatResp::new("failure", "no item found", StatusCode::OK).into_response()
        }
        Err(e) => {
            // e.chain().skip(0).for_each(|cause| println!("because: {}", cause.to_string()));
            dbg!("{:?}", &e);
            return StatResp::new("failure", e.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    };

    // Add modified values to user_table
    user_table.product = update_user_table.product;
    user_table.price = update_user_table.price;

    // Using create_entity_serde because that uses PutItem, which is what we're doing here,
    //  by completely replacing old item.
    return match crate::dynamo_query_helpers::create_entity_serde(&client, user_table, &table).await {
        Ok(_) => {
            axum::response::IntoResponse::into_response(
                StatResp {
                    result: "success".to_string(),
                    message: "updated item".to_string(),
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


pub async fn delete_user_table_serde_rest_handler(
    // axum::extract::Path(user_id): axum::extract::Path<String>
    axum::extract::Path((user, order)): axum::extract::Path<(String, String)>,

) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let table = "UserTable".to_string();


    let key =
        HashMap::from([
            // Map of [ key_field_name, key_field_value_as_attribute_value ]
            (String::from("UserId"), serde_dynamo::to_attribute_value("u#".to_string() + &*user).unwrap()),
            (String::from("OrderId"),  serde_dynamo::to_attribute_value("o#".to_string() + &*order).unwrap()),
        ]);

    match crate::dynamo_query_helpers::delete_by_key_attribute_value_serde(&client, &table, key).await {
        Ok(item) =>
            { StatResp::new("success", "deleted item", StatusCode::OK).into_response() }
        Err(e) => StatResp::new("failure", e.to_string().as_str(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }
}
