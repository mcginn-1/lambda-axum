#![allow(unused)]


// use crate::config::auth::jwk;
// use crate::config::auth::jwk::{JwkConfiguration, JwkKey};
use jsonwebtoken::decode_header;
use jsonwebtoken::TokenData;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use axum::http::StatusCode;
use crate::auth::{AuthError, VerificationError};
use crate::jwk;
// use crate::auth::AuthError;
use crate::jwk::{JwkConfiguration, JwkKey};

#[derive(Debug, Deserialize, Clone)]
pub struct FBTokenClaims {
    // The audience the token was issued for
    pub aud: String,
    // The expiry date -- as epoch seconds
    pub exp: i64,
    // The token issuer
    pub iss: String,
    // The subject the token refers to
    pub sub: String,
    // Issued at -- as epoch seconds
    pub iat: i64,
}

// enum VerificationError {
//     InvalidSignature,
//     UnknownKeyAlgorithm,
// }

#[derive(Debug)]
pub struct JwkVerifier {
    keys: HashMap<String, JwkKey>,
    config: JwkConfiguration,
}

fn keys_to_map(keys: Vec<JwkKey>) -> HashMap<String, JwkKey> {
    let mut keys_as_map = HashMap::new();
    for key in keys {
        keys_as_map.insert(String::clone(&key.kid), key);
    }
    keys_as_map
}

impl JwkVerifier {
    pub fn new(keys: Vec<JwkKey>) -> JwkVerifier {
        JwkVerifier {
            keys: keys_to_map(keys),
            config: jwk::get_configuration(),
        }
    }

    pub fn verify(&self, token: &String) -> Result<TokenData<FBTokenClaims>, AuthError> {

        let token_kid = match decode_header(token).map(|header| header.kid) {
            Ok(Some(header)) => header,
            _ => return Err(AuthError::TokenDecodeError ),
        };

        let jwk_key = match self.get_key(token_kid) {
            Some(key) => key,
            // _ => return None,
            _ => return Err(AuthError::NoTokenKeyError ),
        };


        let token_data = self.decode_token_with_key(jwk_key, token)?;
        Ok(token_data)

        // match self.decode_token_with_key(jwk_key, token) {
        //     Ok(token_data) => Ok(token_data),
        //     Err(err) => return
        //         Err(AuthError{
        //             message: match err {
        //                 VerificationError::InvalidSignature => {"invalid token signature".to_string()}
        //                 VerificationError::UnknownKeyAlgorithm => {"unknown key algorithm".to_string()}
        //             },
        //             status_code: StatusCode::UNAUTHORIZED
        //         })
        //  }

        // match self.decode_token_with_key(jwk_key, token) {
        //     Ok(token_data) => Ok(token_data),
        //     _ => return Err(AuthError{message: "could not decode token".parse().unwrap(), status_code: StatusCode::UNAUTHORIZED}),
        //
        //     // _ => None,
        // }
    }

    // pub fn verify(&self, token: &String) -> Option<TokenData<Claims>> {
    //     let token_kid = match decode_header(token).map(|header| header.kid) {
    //         Ok(Some(header)) => header,
    //         _ => return None,
    //     };
    //
    //     let jwk_key = match self.get_key(token_kid) {
    //         Some(key) => key,
    //         _ => return None,
    //     };
    //
    //     match self.decode_token_with_key(jwk_key, token) {
    //         Ok(token_data) => Some(token_data),
    //         _ => None,
    //     }
    // }

    pub fn set_keys(&mut self, keys: Vec<JwkKey>) {
        self.keys = keys_to_map(keys);
    }

    fn get_key(&self, key_id: String) -> Option<&JwkKey> {
        self.keys.get(&key_id)
    }

    fn decode_token_with_key(
        &self,
        key: &JwkKey,
        token: &String,
    ) -> Result<TokenData<FBTokenClaims>, VerificationError> {
        let algorithm = match Algorithm::from_str(&key.alg) {
            Ok(alg) => alg,
            Err(_error) => return Err(VerificationError::UnknownKeyAlgorithm),
        };

        let mut validation = Validation::new(algorithm);
        validation.set_audience(&[&self.config.audience]);

        // Modified this so validation.iss could be a hashset
        // validation.iss = Some(self.config.issuer.clone());
        // let mut validation_iss = HashSet::new();
        // validation_iss.insert(self.config.issuer.clone());
        // validation.iss = Some(validation_iss);

        validation.set_issuer(&[self.config.issuer.clone()]) ;

        let key = DecodingKey::from_rsa_components(&key.n, &key.e);
        return decode::<FBTokenClaims>(token, &key.unwrap(), &validation)
            .map_err(|_| VerificationError::InvalidSignature);
    }

    // fn decode_token_with_key(
    //     &self,
    //     key: &JwkKey,
    //     token: &String,
    // ) -> Result<TokenData<Claims>, VerificationError> {
    //     let algorithm = match Algorithm::from_str(&key.alg) {
    //         Ok(alg) => alg,
    //         Err(_error) => return Err(VerificationError::UnknownKeyAlgorithm),
    //     };
    //
    //     let mut validation = Validation::new(algorithm);
    //     validation.set_audience(&[&self.config.audience]);
    //     validation.iss = Some(self.config.issuer.clone());
    //     let key = DecodingKey::from_rsa_components(&key.n, &key.e);
    //     return decode::<Claims>(token, &key, &validation)
    //         .map_err(|_| VerificationError::InvalidSignature);
    // }
}
