use std::collections::HashMap;
use aws_sdk_dynamodb::{Client, Error};
use aws_sdk_dynamodb::types::AttributeValue;
use serde::{Deserialize, Serialize};
use crate::dynamo::DynamoError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    // pub p_type: String,
    pub account_type: String,
    pub age: String,
    // Key field
    pub username: String,
    pub first_name: String,
    pub last_name: String,
}



#[derive(Debug, PartialEq)]
pub struct ItemOut {
    pub p_type: Option<AttributeValue>,
    pub age: Option<AttributeValue>,
    pub username: Option<AttributeValue>,
    pub first_name: Option<AttributeValue>,
    pub last_name: Option<AttributeValue>,
}


// Used only in dynamo2
pub async fn query_items_scan_serde(
    client: &Client,
    table_name: &str,
    username: &str,

) -> anyhow::Result<Vec<Item>, DynamoError> {
    // Get documents from DynamoDB
    let result = client
        .scan()
        .table_name(table_name)
        .send()
        .await
        .map_err(|e| {
            let se = e.into_service_error();
            println!("{}", se.to_string());
            DynamoError::DynErrorExp {exp: se.to_string()}
        })?;

    // And deserialize them as strongly-typed data structures
    let items = result.items().to_vec();
    let users = serde_dynamo::aws_sdk_dynamodb_1::from_items(items)
        .map_err(|e| {
            println!("{}", e.to_string());
            DynamoError::DynErrorExp {exp: e.to_string()}
            // DynamoError::DynError
        })?;
    println!("Got {} users", users.len());
    Ok(users)
}

// List your tables 10 at a time.
// snippet-start:[dynamodb.rust.list-more-tables]
pub async fn list_tables_iterative(client: &Client) -> anyhow::Result<Vec<String>, Error> {
    let mut resp = client.list_tables().limit(10).send().await?;
    let mut names = resp.table_names.unwrap_or_default();
    let len = names.len();

    let mut num_tables = len;

    println!("Tables:");

    for name in &names {
        println!("  {}", name);
    }

    while resp.last_evaluated_table_name.is_some() {
        println!("-- more --");
        resp = client
            .list_tables()
            .limit(10)
            .exclusive_start_table_name(
                resp.last_evaluated_table_name
                    .as_deref()
                    .unwrap_or_default(),
            )
            .send()
            .await?;

        let mut more_names = resp.table_names.unwrap_or_default();
        num_tables += more_names.len();

        for name in &more_names {
            println!("  {}", name);
        }
        names.append(&mut more_names);
    }

    println!();
    println!("Found {} tables", num_tables);

    Ok(names)
}




/// Serde dynamo - https://docs.rs/serde_dynamo/latest/serde_dynamo/aws_sdk_dynamodb_1/index.html
///
/// Item - dynamo_2
///
pub async fn query_items_by_field_attribute_serde(
    client: &Client,
    table_name: &str,
    username: &str,
) -> anyhow::Result<Vec<Item>, DynamoError> {

    let mut hm: Option<HashMap<String, AttributeValue>> = Some(HashMap::from([
        ("username".to_string(), AttributeValue::S("user4".to_string()) )
    ]));


    let results = client
        .query()
        .table_name(table_name)

        .key_condition_expression("#username = :username")
        .expression_attribute_names("#username", "username")
        .expression_attribute_values(":username", AttributeValue::S(username.to_string()))

        // .key_condition_expression("username = :u" )
        // .expression_attribute_values(":u", AttributeValue::S("user1".to_string()))

        .send()
        .await
        .map_err(|e| {
            println!("{}", e.as_service_error().unwrap().to_string());
            DynamoError::DynErrorExp {exp: e.as_service_error().unwrap().to_string()}
            // DynamoError::DynError
        })?;

    println!("{:?}", results.last_evaluated_key);

    if let Some(items) = results.items {
        // let items = results.items().to_vec();
        let items = items.to_vec();
        let users: Vec<Item> = serde_dynamo::aws_sdk_dynamodb_1::from_items(items)
            .map_err(|e| DynamoError::DynErrorExp {exp: e.to_string()})?;
        println!("Got {} users", users.len());
        Ok(users)
    } else {
        Ok(vec![])
    }
}

/// Serde dynamo - https://docs.rs/serde_dynamo/latest/serde_dynamo/aws_sdk_dynamodb_1/index.html
///
/// Item - dynamo_2
///
pub async fn query_items_by_field_scan_paginate_serde(
    client: &Client,
    table_name: &str,
    username: &str,
) -> anyhow::Result<Vec<Item>, anyhow::Error> {

    let mut hm: Option<HashMap<String, AttributeValue>> = Some(HashMap::from([
        ("username".to_string(), AttributeValue::S("user4".to_string()) )
    ]));

    let results = client
        .scan()
        .table_name(table_name)
        .limit(2)
        .set_exclusive_start_key(hm)
        .send()
        .await
        .map_err(|e| {
            e.into_service_error()
        })?;
    // .map_err(|e| {
    //     anyhow!(e.as_service_error().unwrap().to_string())
    // })?;


    println!("{:?}", results.last_evaluated_key);

    if let Some(items) = results.items {
        let items = items.to_vec();
        let users: Vec<Item> = serde_dynamo::aws_sdk_dynamodb_1::from_items(items)?;
        println!("Got {} users", users.len());
        Ok(users)
    } else {
        Ok(vec![])
    }
}



// Query Item by username - Non-Serde
pub async fn query_items_by_username(
    client: &Client,
    table_name: &str,
    username: &str,
) -> anyhow::Result<Vec<Item>, Error> {

    let results = client
        .query()
        .table_name(table_name)

        // .key_condition_expression("#username = :username")
        // .expression_attribute_names("#username", "username")
        // .expression_attribute_values(":username", AttributeValue::S(username.to_string()))

        .key_condition_expression("username = :u" )
        .expression_attribute_values(":u", AttributeValue::S("user1".to_string()))

        // .key_condition_expression("username = :u and age = :a")
        // .expression_attribute_values(":u", AttributeValue::S("user1".to_string()))
        // .expression_attribute_values(":a", AttributeValue::S("22".to_string()))

        .send()
        .await?;


    if let Some(items) = results.items {
        // let movies = items.iter().map(|v| v.into()).collect();
        let movies = items
            .iter()
            .map(|attributes|

                {
                    let username = attributes.get("username").cloned();
                    let first_name = attributes.get("first_name").cloned();
                    let last_name = attributes.get("last_name").cloned();
                    let age = attributes.get("age").cloned();
                    let p_type = attributes.get("account_type").cloned();

                    println!(
                        "Added user {:?}, {:?} {:?}, age {:?} as {:?} user",
                        username, first_name, last_name, age, p_type
                    );

                    // attributes
                    //     .get("some_column_name")
                    //     .ok_or("missing field")?
                    //     .as_s()
                    //     .map_err(|value| format!("expected a string, found: {:?}", value))?;


                    let binding = String::new();

                    let username2 = attributes.get("username")
                        .unwrap()
                        .as_s()
                        .unwrap_or(&binding);
                    // .map_err(|value| format!("expected a string, found: {:?}", value))?;

                    Item{
                        // p_type: p_type.unwrap().as_s().unwrap_or(&binding).to_string(),
                        account_type: p_type.unwrap().as_s().unwrap_or(&binding).to_string(),
                        age: age.unwrap().as_s().unwrap_or(&binding).to_string(),
                        username: username2.clone(),
                        first_name: first_name.unwrap().as_s().unwrap_or(&binding).to_string(),
                        last_name: last_name.unwrap().as_s().unwrap_or(&binding).to_string(),
                    }
                }


            )

            .collect();
        Ok(movies)
    } else {
        Ok(vec![])
    }
}

// Lists the items in a table.
// snippet-start:[dynamodb.rust.list-items]
pub async fn list_items(client: &Client, table: &str, page_size: Option<i32>) -> Result<(), Error> {
    let page_size = page_size.unwrap_or(10);
    let items: Result<Vec<_>, _> = client
        .scan()
        .table_name(table)
        .limit(page_size)
        .into_paginator()
        .items()
        .send()
        .collect()
        .await;

    println!("Items in table (up to {page_size}):");
    for item in items? {
        println!("   {:?}", item);
    }

    Ok(())
}

// pub async fn query2(
//     client: &Client,
//     table_name: &str,
//     year: u16,
// ) -> Result<Vec<Item>, DynamoError> {
//     let results = client
//         .query()
//         .table_name(table_name)
//         .key_condition_expression("#yr = :yyyy")
//         .expression_attribute_names("#yr", "year")
//         .expression_attribute_values(":yyyy", AttributeValue::N(year.to_string()))
//         .send()
//         .await?;
//
//     if let Some(items) = results.items {
//         let movies = items.iter().map(|v| v.into()).collect();
//         Ok(movies)
//     } else {
//         Ok(vec![])
//     }
// }


// Used in Item - dynamo_2
pub async fn add_item_serde(client: &Client, item: Item, table: &String)
                            -> Result<(), anyhow::Error> {

    // Turn it into an item that aws-sdk-dynamodb understands
    let item = serde_dynamo::aws_sdk_dynamodb_1::to_item(item)?;

    // Write item to db
    client
        .put_item()
        .table_name(table)
        .set_item(Some(item))
        .send()
        .await
        .map_err(|e| {
            e.into_service_error()
        })?;

    Ok(())
}

// Add item non-serde - Item dynamo_2
pub async fn add_item(client: &Client, item: Item, table: &String) -> Result<(), anyhow::Error> {

    let user_av = AttributeValue::S(item.username);
    // let type_av = AttributeValue::S(item.p_type);
    let type_av = AttributeValue::S(item.account_type);
    let age_av = AttributeValue::S(item.age);
    let first_av = AttributeValue::S(item.first_name);
    let last_av = AttributeValue::S(item.last_name);

    let request = client
        .put_item()
        .table_name(table)
        // .return_values(ReturnValue::AllOld)
        .item("username", user_av)
        .item("account_type", type_av)
        .item("age", age_av)
        .item("first_name", first_av)
        .item("last_name", last_av);


    println!("Executing request [{request:?}] to add item...");

    let resp = request.send().await?;


    // Attributes will only appear if request specifies all old:
    //  .return_values(ReturnValue::AllOld)
    match resp.attributes() {
        None => {
            println!("NO ATTRIBUTES");
            // Err(Error::BackupInUseException)
            Ok(())

        }
        Some(attributes) => {
            let username = attributes.get("username").cloned();
            let first_name = attributes.get("first_name").cloned();
            let last_name = attributes.get("last_name").cloned();
            let age = attributes.get("age").cloned();
            let p_type = attributes.get("p_type").cloned();

            println!(
                "Added user {:?}, {:?} {:?}, age {:?} as {:?} user",
                username, first_name, last_name, age, p_type
            );

            Ok(())


            // Ok(ItemOut {
            //     p_type,
            //     age,
            //     username,
            //     first_name,
            //     last_name,
            // })
            //
        }
    }


}
