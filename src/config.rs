use crate::Result;
use std::env::var;

pub struct Config {
    pub token: String,
    pub test_guild: Option<u64>,
    pub app_id: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let token = var("TOKEN")?;
        let app_id = var("APP_ID")?.parse()?;
        let test_guild: Option<u64> = {
            match var("TEST_GUILD") {
                Ok(s) => {
                    let v: u64 = s.parse()?;
                    Some(v)
                }
                Err(e) => return Err(e.into()),
            }
        };
        Ok(Config {
            token,
            test_guild,
            app_id,
        })
    }
}
