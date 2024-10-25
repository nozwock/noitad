use std::path::PathBuf;

use crate::NOITA_STEAM_ID;

pub enum NoitaPath {
    Steam,
    GameRoot(Option<PathBuf>),
}

impl Default for NoitaPath {
    fn default() -> Self {
        let steam_app_found = steamlocate::SteamDir::locate()
            .map(|ref mut it| it.app(&NOITA_STEAM_ID).is_some())
            .unwrap_or_default();

        match steam_app_found {
            true => Self::Steam,
            false => Self::GameRoot(None),
        }
    }
}

impl NoitaPath {
    pub fn game_root(self) -> Option<PathBuf> {
        match self {
            NoitaPath::Steam => steamlocate::SteamDir::locate()
                .as_mut()
                .map(|it| it.app(&NOITA_STEAM_ID).map(|it| it.path.clone()))
                .flatten(),
            NoitaPath::GameRoot(path) => path,
        }
    }
    pub fn workshop(self) -> Option<PathBuf> {
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
    pub fn local_mods(self) -> Option<PathBuf> {
        self.game_root().map(|p| p.join("mods"))
    }
}
