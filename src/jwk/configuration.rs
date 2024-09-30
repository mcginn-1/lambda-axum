#![allow(unused)]


use std::env;

#[derive(Debug)]
pub struct JwkConfiguration {
    pub jwk_url: String,
    pub audience: String,
    pub issuer: String,
}

// #[cfg(debug_assertions)]
fn expect_env_var(name: &str, default: &str) -> String {
    return env::var(name).unwrap_or(String::from(default));
}

pub fn get_configuration() -> JwkConfiguration {

    JwkConfiguration {
        jwk_url: expect_env_var("JWK_URL", ""),
        audience: expect_env_var("JWK_AUDIENCE", ""),
        issuer: expect_env_var("JWK_ISSUER", ""),
    }

}
