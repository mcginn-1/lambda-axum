use std::collections::HashMap;
use std::future::Future;
use std::str::FromStr;
use aliri_braid::braid;
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::operation::create_table::{CreateTableError, CreateTableOutput};
use modyne::{expr, keys, types::Expiry, Aggregate, Entity, EntityDef, EntityExt, Error, Projection, ProjectionExt, QueryInput, QueryInputExt, Table, EntityTypeNameRef};

#[derive(Clone, Debug)]
pub struct App {
    table_name: std::sync::Arc<str>,
    client: aws_sdk_dynamodb::Client,
}

impl App {
    pub fn new(client: aws_sdk_dynamodb::Client) -> Self {
        Self::new_with_table(client, "SessionStore")
    }

    pub fn new_with_table(client: aws_sdk_dynamodb::Client, table_name: &str) -> Self {
        Self {
            table_name: std::sync::Arc::from(table_name),
            client,
        }
    }
}

impl Table for App {
    /// For demonstration, this example uses a non-standard entity type attribute name
    const ENTITY_TYPE_ATTRIBUTE: &'static str = "et";

    type PrimaryKey = SessionToken;
    type IndexKeys = UsernameKey;

    fn table_name(&self) -> &str {
        &self.table_name
    }

    fn client(&self) -> &aws_sdk_dynamodb::Client {
        &self.client
    }
}

impl App {
    pub async fn create_session(&self, session: Session) -> Result<(), Error> {
        session.create().execute(self).await?;
        Ok(())
    }

    pub async fn delete_session(&self, uuid: Uuid) -> Result<(), Error> {
        Session::delete(uuid).execute(self).await?;
        Ok(())
    }


    pub async fn update_session(&self, session: Session) -> Result<(), Error> {
        Session::replace(session).execute(self).await?;
        Ok(())
    }


    pub async fn get_any_session(
        &self,
        session_token: uuid::Uuid,
    ) -> Result<Option<Session>, Error> {
        let result = Session::get(session_token).execute(self).await?;
        if let Some(item) = result.item {
            let session = Session::from_item(item)?;
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    pub async fn get_session(&self, session_token: uuid::Uuid) -> Result<Option<Session>, Error> {
        let now = time::OffsetDateTime::now_utc();
        self.get_session_with_now(session_token, now).await
    }

    pub async fn get_session_with_now(
        &self,
        session_token: uuid::Uuid,
        now: time::OffsetDateTime,
    ) -> Result<Option<Session>, Error> {
        let result = Session::get(session_token).execute(self).await?;
        if let Some(item) = result.item {
            let session = Session::from_item(item)?;
            if session.expires_at > now {
                Ok(Some(session))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    // pub async fn delete_user_sessions(&self, user: &UsernameRef) -> Result<(), Error> {
    //     let mut joiner = tokio::task::JoinSet::new();
    //     loop {
    //         let mut agg = Vec::<SessionTokenOnly>::new();
    //
    //         let result = user.query().execute(self).await?;
    //
    //         agg.reduce(result.items.unwrap_or_default())?;
    //
    //         for session in agg {
    //             let this = self.clone();
    //             joiner.spawn(
    //                 async move { Session::delete(session.session_token).execute(&this).await },
    //             );
    //         }
    //
    //         if result.last_evaluated_key.is_none() {
    //             break;
    //         }
    //     }
    //
    //     let mut last_result = Ok(());
    //
    //     while let Some(next) = joiner.join_next().await {
    //         match next {
    //             Ok(Ok(_)) => {}
    //             Ok(Err(err)) => {
    //                 tracing::error!(
    //                     exception = &err as &dyn std::error::Error,
    //                     "error while deleting session"
    //                 );
    //                 last_result = Err(err);
    //             }
    //             Err(err) => {
    //                 tracing::error!(
    //                     exception = &err as &dyn std::error::Error,
    //                     "panic while deleting session"
    //                 );
    //             }
    //         }
    //     }
    //
    //     Ok(last_result?)
    // }
}

#[braid(serde)]
pub struct Username;

#[derive(Clone, Debug, serde::Serialize)]
pub struct SessionToken {
    pub session_token: uuid::Uuid,
}

impl keys::PrimaryKey for SessionToken {
    const PRIMARY_KEY_DEFINITION: keys::PrimaryKeyDefinition = keys::PrimaryKeyDefinition {
        hash_key: "session_token",
        range_key: None,
    };
}

impl keys::Key for SessionToken {
    const DEFINITION: keys::KeyDefinition =
        <Self as keys::PrimaryKey>::PRIMARY_KEY_DEFINITION.into_key_definition();
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct UsernameKey {
    pub username: Username,
}

impl keys::IndexKey for UsernameKey {
    const INDEX_DEFINITION: keys::SecondaryIndexDefinition = keys::GlobalSecondaryIndexDefinition {
        index_name: "UserIndex",
        hash_key: "username",
        range_key: None,
    }
        .into_index();
}

// pub use modyne_derive::EntityDef;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub session_token: uuid::Uuid,
    pub username: Username,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: time::OffsetDateTime,
    pub ttl: Expiry,
}

impl Entity for Session {
    type KeyInput<'a> = uuid::Uuid;
    type Table = App;
    type IndexKeys = UsernameKey;

    fn primary_key(input: Self::KeyInput<'_>) -> SessionToken {
        SessionToken {
            session_token: input,
        }
    }

    fn full_key(&self) -> keys::FullKey<SessionToken, Self::IndexKeys> {
        keys::FullKey {
            primary: Self::primary_key(self.session_token),
            indexes: UsernameKey {
                username: self.username.clone(),
            },
        }
    }
}

impl EntityDef for Session {
    // const ENTITY_TYPE: &'static EntityTypeNameRef = &EntityTypeNameRef(());
    // const PROJECTED_ATTRIBUTES: &'static [&'static str] = &[];

    const ENTITY_TYPE: &'static EntityTypeNameRef =
        EntityTypeNameRef::from_static("Session");

    const PROJECTED_ATTRIBUTES: &'static [&'static str] = &[
        "session_token",
        "username",
        "created_at",
        "expires_at",
        "ttl",
    ];
}


pub async fn get_session_modyne_handler(
    axum::extract::Path((session_id)): axum::extract::Path<(String)>,
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let app = App::new(client);

    match get_session_modyne(app, session_id.as_str()).await {
        Ok(session) => match session {
            Some(session) => {axum::Json(session).into_response()}
            None => {
                StatResp::new("failure",
                              "not found",
                              StatusCode::OK).into_response()
            }
        }
        Err(e) => {
            dbg!("{:?}", &e);
            StatResp::new("failure",
                          e.to_string().as_str(),
                          StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn get_session_modyne(app: App, uuid_str: &str) -> Result<Option<Session>, anyhow::Error> {
    let session = app
            .get_any_session(uuid::Uuid::from_str(uuid_str).unwrap())
            .await?;
    Ok(session)
}



pub async fn create_session_modyne_handler(
    // axum::extract::Path((session_id)): axum::extract::Path<(String)>,
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let app = App::new(client);

    match create_session_modyne(app).await {
        Ok(session_token) => {
            StatResp::new("success",
                          session_token.to_string().as_str(),
                          StatusCode::OK).into_response()
        }

        Err(e) => {
            dbg!("{:?}", &e);
            StatResp::new("failure",
                          e.to_string().as_str(),
                          StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

// pub async fn create_session_modyne(app: App, uuid_str: &str) -> Result<uuid::Uuid, anyhow::Error> {
pub async fn create_session_modyne(app: App) -> Result<uuid::Uuid, anyhow::Error> {
    let session_token =  uuid::Uuid::new_v4();
    let session = Session {
        session_token: session_token,
        username: Username::from(format!("mtest_{}", 3 % 13)),
        created_at: time::OffsetDateTime::now_utc(),
        expires_at: time::OffsetDateTime::now_utc(),
        ttl: Expiry::from(time::OffsetDateTime::now_utc()),
    };
    app.create_session(session).await?;
    Ok(session_token)
}


/// Updates the session username - Looks up the session by id,
/// Changes the username and updates the whole Session
pub async fn update_session_username_modyne_handler(
    axum::extract::Path((session_id, username)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let app = App::new(client);

    match crate::modyne::update_session_username_modyne(app, session_id.clone(), username).await {
        Ok(session_token) => {
            StatResp::new("success",
                          format!("updated item: {}", session_id.as_str()).as_str(),
                          StatusCode::OK).into_response()
        }

        Err(e) => {
            dbg!("{:?}", &e);
            StatResp::new("failure",
                          e.to_string().as_str(),
                          StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn update_session_username_modyne(app: App, session_id: String, username: String) -> Result<(), anyhow::Error> {
    let session_query_result = get_session_modyne(app.clone(), session_id.as_str()).await?;

    if let Some(mut session) = session_query_result {
        session.username = Username::from(username);
        update_session_modyne(app, session).await?;
        Ok(())
    } else {
        Err(DynErrorExp {exp: "None found".to_string()})?
    }
}

/// Update the session username
pub async fn update_session_modyne(app: App, session: Session) -> Result<(), anyhow::Error> {
    // let uuid = Uuid::from_str(uuid_str)?;
    app.update_session(session).await?;
    Ok(())
}


pub async fn delete_session_modyne_handler(
    axum::extract::Path((session_id)): axum::extract::Path<(String)>,
) -> impl IntoResponse {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let app = App::new(client);

    match crate::modyne::delete_session_modyne(app, session_id.as_str()).await {
        Ok(session_token) => {
            StatResp::new("success",
                          format!("deleted item: {}", session_id.as_str()).as_str(),
                          StatusCode::OK).into_response()
        }

        Err(e) => {
            dbg!("{:?}", &e);
            StatResp::new("failure",
                          e.to_string().as_str(),
                          StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn delete_session_modyne(app: App, uuid_str: &str) -> Result<(), anyhow::Error> {
    let uuid = Uuid::from_str(uuid_str)?;
    app.delete_session(uuid).await?;
    Ok(())
}

pub async fn blah() -> impl IntoResponse{
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);

    let app = App::new(client);

    match app.get_any_session(uuid::Uuid::from_str("07b2bc80-1caa-400b-9aea-090819f49937").unwrap())
        .await {
            Ok(session) => {{println!("Session Output: {:?}", session)}}
            Err(e) => {println!("ERROR {:?}", e)}
    }



}


use aws_sdk_dynamodb::types::TimeToLiveSpecification;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use lambda_runtime::IntoFunctionResponse;
use modyne::{

    model::{BatchGet, BatchWrite},

  TestTableExt,
};
use uuid::{uuid, Uuid};
use crate::dynamo::DynamoError::{DynError, DynErrorExp};
use crate::dynamo::StatResp;
use crate::dynamo_query_helpers::{query_items_key_attribute_value_serde};


