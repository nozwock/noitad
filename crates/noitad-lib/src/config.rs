use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::noita::NoitaPath;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    noita_path: NoitaPath,
    wine_prefix: Option<PathBuf>,
}
