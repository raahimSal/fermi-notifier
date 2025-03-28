// src/config.rs
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub gemini_api_key: String,
    pub ntfy_topic: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        // In Cloud Run, PORT is set automatically. For local, we use .env
        let port_str = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
        let port = port_str.parse::<u16>().expect("PORT must be a number");

        Ok(Self {
            gemini_api_key: env::var("GEMINI_API_KEY")?,
            ntfy_topic: env::var("NTFY_TOPIC")?,
            port,
        })
    }
}
