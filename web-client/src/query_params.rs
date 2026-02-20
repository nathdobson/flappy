use crate::error::Error;
use crate::utils::try_window;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlappyQueryParams {
    pub ws_url: String,
    pub username: String,
    pub password: String,
    pub topic: String,
    #[serde(default)]
    pub spindle: bool,
}

impl FlappyQueryParams {
    pub fn new() -> Result<Self, Error> {
        let search = try_window()?.location().search()?;
        let search = search.strip_prefix("?").unwrap_or(&search);
        Ok(serde_qs::from_str(&search)?)
    }
}
