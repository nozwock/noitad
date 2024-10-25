use color_eyre::eyre::{bail, Result};
use confy::ConfyError;
use fs_err as fs;
use serde::{Deserialize, Serialize};

use crate::{defines::APP_CONFIG_PATH, noita::NoitaPath};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    noita_path: NoitaPath,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            noita_path: Default::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let cfg = match confy::load_path::<Self>(APP_CONFIG_PATH.as_path()) {
            Ok(cfg) => cfg,
            Err(ConfyError::BadTomlData(_err)) => {
                fs::remove_file(APP_CONFIG_PATH.as_path())?;
                confy::load_path::<Self>(APP_CONFIG_PATH.as_path())?
            }
            Err(err) => bail!(err),
        };
        Ok(cfg)
    }
    pub fn store(self) -> Result<()> {
        confy::store_path(APP_CONFIG_PATH.as_path(), self)?;
        Ok(())
    }
}
