use crate::Result;
use std::env::var;

pub struct Config {
    pub token: String,
    pub app_id: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let token = var("TOKEN")?;
        let app_id = var("APP_ID")?.parse()?;
        Ok(Config { token, app_id })
    }
}
