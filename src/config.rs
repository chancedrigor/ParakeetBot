use crate::error::Error;
use crate::Result;
use std::env::var as std_var;

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
                Err(_) => None,
            }
        };
        Ok(Config {
            token,
            test_guild,
            app_id,
        })
    }
}

fn var(name: impl AsRef<str>) -> Result<String> {
    let var_name = name.as_ref();
    std_var(var_name).map_err(|_e| Error::MissingEnv(var_name.to_string()).into())
}
