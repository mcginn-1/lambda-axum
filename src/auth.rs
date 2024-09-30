#![allow(unused)]


use axum::{
    body::Body,
    response::IntoResponse,
    extract::{Request, Json},
    http,
    http::{Response, StatusCode},
    middleware::Next,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{jwk};
// use crate::dynamo::Paginator;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Empty header is not allowed")]
    EmptyHeaderError ,
    #[error("Please add the JWT token to the header")]
    MissingAuthHeaderError,
    #[error("Error decoding JWT")]
    TokenDecodeError,
    #[error("Unauthorized user")]
    UnauthorizedUserError,
    #[error("Couldn't find user by email")]
    NoUserError,
    #[error("Could not obtain token key")]
    NoTokenKeyError,
    #[error("{0}")]
    JWTVerificationError (#[from] VerificationError),

    // Creating JWT
    #[error("BcryptError")]
    BCryptError,
    #[error("Passwords don't match")]
    PasswordError,
    #[error("Could not generate JWT")]
    GenerateJWTError,
}

#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    #[error("Invalid token signature")]
    InvalidSignature,
    #[error("Unknown key algorithm")]
    UnknownKeyAlgorithm,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::http::Response<Body> {

        let result = self.to_string();
        let body = Json(json!({
            "error": result,
        }));

        (StatusCode::UNAUTHORIZED, body).into_response()
    }
}


// From JWT Tutorial https://blog.logrocket.com/using-rust-axum-build-jwt-authentication-api/

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
    pub email: String,
}


pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    let hash = hash(password, DEFAULT_COST)?;
    Ok(hash)
}

pub fn encode_jwt(email: String) -> Result<String, StatusCode> {
    let jwt_token: String = "randomstring".to_string();

    let now = Utc::now();
    let expire: chrono::TimeDelta = Duration::hours(24);
    let exp: usize = (now + expire).timestamp() as usize;
    let iat: usize = now.timestamp() as usize;

    let claim = Claims { iat, exp, email };
    let secret = jwt_token.clone();

    encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(secret.as_ref()),
    )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn decode_jwt(jwt: String) -> Result<TokenData<Claims>, StatusCode> {
    let secret = "randomstring".to_string();

    let result: Result<TokenData<Claims>, StatusCode> = decode(
        &jwt,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    result
}

#[derive(Clone)]
pub struct CurrentUser {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub password_hash: String
}

/// The authorize middleware is getting the current user from
///  the token and placing the user in an extension
///  that is consumed by the function that is called
///
///  check out docs:  https://docs.rs/axum/latest/axum/middleware/index.html#passing-state-from-middleware-to-handlers
///
pub async fn authorize(mut req: Request, next: Next) -> Result<Response<Body>, AuthError> {
    let auth_header = req.headers_mut().get(http::header::AUTHORIZATION);

    let auth_header = match auth_header {
        Some(header) => header.to_str()
            .map_err(|_| AuthError::EmptyHeaderError)?,
        None => Err(AuthError::MissingAuthHeaderError)?,
    };

    let mut header = auth_header.split_whitespace();

    // Splitting 'Bearer' from token
    let (bearer, token) = (header.next(), header.next());


    // this uses the homegrown JWT decoder
    let token_data = match decode_jwt(token.unwrap().to_string()) {
        Ok(data) => data,
        Err(_) => return Err(AuthError::TokenDecodeError),
    };

    // Fetch the user details from the database
    let current_user = match retrieve_user_by_email(&token_data.claims.email) {
        Some(user) => user,
        None => return Err(AuthError::NoUserError),
    };

    req.extensions_mut().insert(current_user);
    Ok(next.run(req).await)
}



/// No Longer Used
// pub async fn get_paginator_token(mut req: Request, next: Next) -> Result<Response<Body>, AuthError> {
//     let token = match req.headers()
//         .get("app_token") {
//         Some(value) => {
//
//             match value.to_str() {
//             Ok(token) => {Some(Paginator {token: token.to_string()})}
//             Err(e) => {Err(AuthError::MissingAuthHeaderError)?}
//         }}
//         None => {None}
//     };
//     req.extensions_mut().insert(token);
//     Ok(next.run(req).await)
// }

/// The authorize middleware is getting the current user from
///  the token and placing the user in an extension
///  that is consumed by the function that is called
///
///  check out docs:  https://docs.rs/axum/latest/axum/middleware/index.html#passing-state-from-middleware-to-handlers
///
pub async fn authorize_firebase(mut req: Request, next: Next) -> Result<Response<Body>, AuthError> {
    let auth_header = req.headers_mut().get(http::header::AUTHORIZATION);

    let auth_header = match auth_header {
        Some(header) => header.to_str()
            .map_err(|_|
                AuthError::EmptyHeaderError
            )?,
        None => Err(AuthError::MissingAuthHeaderError)?,
    };

    let mut header = auth_header.split_whitespace();

    // Splitting 'Bearer' from token
    let (bearer, token) = (header.next(), header.next());

    let firebase_token_data = jwk::JwkAuth::new()
        .verify_firebase_jwt(&token.unwrap().to_string())?;

    // let current_user: CurrentUser = CurrentUser {
    //     email: firebase_token_data.claims.sub,
    //     first_name: "Eze".to_string(),
    //     last_name: "Sunday".to_string(),
    //     // the plain password hashed to this is "okon" without the quotes.
    //     password_hash: "$2b$12$Gwf0uvxH3L7JLfo0CC/NCOoijK2vQ/wbgP.LeNup8vj6gg31IiFkm".to_string()
    // };


    // // // Fetch the user details from the database
    // let current_user: CurrentUser = match retrieve_user_by_email(&token_data.claims.email) {
    //     Some(user) => user,
    //     None => return Err(AuthError {
    //         message: "You are not an authorized user".to_string(),
    //         status_code: StatusCode::UNAUTHORIZED
    //     }),
    // };

    req.extensions_mut().insert(firebase_token_data);
    // req.extensions_mut().insert(current_user);
    Ok(next.run(req).await)
}

#[derive(Deserialize)]
pub struct SignInData {
    pub email: String,
    pub password: String,
}

pub async fn sign_in(
    Json(user_data): Json<SignInData>,
) -> Result<Json<String>, AuthError> {

    // 1. Retrieve user from the database
    let user = match retrieve_user_by_email(&user_data.email) {
        Some(user) => user,
        // None => return Err(StatusCode::UNAUTHORIZED), // User not found
        None => return Err(AuthError::NoUserError), // User not found
    };

    // 2. Compare the password
    if !verify_password(&user_data.password, &user.password_hash)
        .map_err(|_| AuthError::BCryptError)? // Handle bcrypt errors
    {
        // passwords dont match
        return Err(AuthError::PasswordError); // Wrong password
    }

    // 3. Generate JWT
    let token = encode_jwt(user.email)
        .map_err(|_| AuthError::GenerateJWTError)?;

    println!("Token: {}", token);

    // 4. Return the token
    Ok(Json(token))
}

fn retrieve_user_by_email(email: &str) -> Option<CurrentUser> {
    let current_user: CurrentUser = CurrentUser {
        email: "myemail@gmail.com".to_string(),
        first_name: "Eze".to_string(),
        last_name: "Sunday".to_string(),
        // the plain password hashed to this is "okon" without the quotes.
        password_hash: "$2b$12$Gwf0uvxH3L7JLfo0CC/NCOoijK2vQ/wbgP.LeNup8vj6gg31IiFkm".to_string()
    };
    Some(current_user)
}
