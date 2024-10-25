use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::defines::NOITA_STEAM_ID;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePath {
    game_root: PathBuf,
    wine_prefix: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoitaPath {
    Steam,
    Other(Option<GamePath>),
}

impl Default for NoitaPath {
    fn default() -> Self {
        let steam_app_found = steamlocate::SteamDir::locate()
            .map(|ref mut it| it.app(&NOITA_STEAM_ID).is_some())
            .unwrap_or_default();

        match steam_app_found {
            true => Self::Steam,
            false => Self::Other(None),
        }
    }
}

impl NoitaPath {
    pub fn game_root(&self) -> Option<PathBuf> {
        match self {
            NoitaPath::Steam => steamlocate::SteamDir::locate()
                .as_mut()
                .map(|it| it.app(&NOITA_STEAM_ID).map(|it| it.path.clone()))
                .flatten(),
            NoitaPath::Other(game_path) => game_path.as_ref().map(|it| it.game_root.clone()),
        }
    }
    pub fn save_dir(&self) -> Option<PathBuf> {
        let appdata_part = "AppData/LocalLow/Nolla_Games_Noita/save00";
        match self {
            NoitaPath::Steam => steamlocate::SteamDir::locate()
                .as_mut()
                .map(|it| {
                    it.libraryfolders()
                        .paths
                        .iter()
                        .map(|library: &PathBuf| {
                            library
                                .join("compatdata")
                                .join(&NOITA_STEAM_ID.to_string())
                                .join("pfx/drive_c/users/steamuser")
                                .join(appdata_part)
                        })
                        .filter(|it| it.is_dir())
                        .next()
                })
                .flatten(),
            NoitaPath::Other(game_path) => {
                if cfg!(target_os = "windows") {
                    directories::UserDirs::new()
                        .map(|it| it.home_dir().join(appdata_part))
                        .filter(|it| it.is_dir())
                } else if cfg!(target_os = "linux") {
                    game_path
                        .as_ref()
                        .map(|it| it.wine_prefix.clone())
                        .flatten()
                        .map(|path| {
                            WalkDir::new(path.join("drive_c/users"))
                                .follow_links(true)
                                .max_depth(1)
                                .min_depth(1)
                                .into_iter()
                                .filter_entry(|e| {
                                    e.file_name() != "Public" || e.file_name() != "steamuser"
                                })
                                .find_map(|it| it.map(|e| e.into_path()).ok())
                        })
                        .flatten()
                        .map(|p| p.join(appdata_part))
                        .filter(|p| p.is_dir())
                } else {
                    unimplemented!()
                }
            }
        }
    }
    pub fn workshop(&self) -> Option<PathBuf> {
        match self {
            NoitaPath::Steam => steamlocate::SteamDir::locate()
                .as_mut()
                .map(|it| {
                    it.libraryfolders()
                        .paths
                        .iter()
                        .map(|library| {
                            library
                                .join("workshop/content")
                                .join(&NOITA_STEAM_ID.to_string())
                        })
                        .filter(|path| path.is_dir())
                        .next()
                })
                .flatten(),
            _ => None,
        }
    }
    pub fn local_mods(&self) -> Option<PathBuf> {
        self.game_root().map(|p| p.join("mods"))
    }
}
