use color_eyre::eyre::{self, Result};
use serde::{Deserialize, Serialize};

use crate::{
    defines::APP_CONFIG_PATH,
    noita::{ModProfiles, NoitaPath},
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    pub noita_path: NoitaPath,
    pub profiles: ModProfiles,
    pub active_profile: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        confy::load_path::<Self>(APP_CONFIG_PATH.as_path()).map_err(eyre::Report::msg)
    }
    pub fn store(&self) -> Result<()> {
        confy::store_path(APP_CONFIG_PATH.as_path(), &self)?;
        Ok(())
    }
}
