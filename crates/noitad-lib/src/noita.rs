pub mod mod_config;
use fs_err as fs;

use std::{
    collections::HashMap,
    io::Write,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use color_eyre::eyre::{bail, ContextCompat, Result};
use mod_config::Mods;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::defines::{MOD_PROFILES_DIR, NOITA_STEAM_ID};

/// HashMap of profile names and filepath to their mod_config file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModProfiles(pub HashMap<String, PathBuf>);

impl Deref for ModProfiles {
    type Target = HashMap<String, PathBuf>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ModProfiles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ModProfiles {
    pub fn add_profile(
        &mut self,
        profile: impl AsRef<str>,
        noita_save_dir: impl AsRef<Path>,
    ) -> Result<Mods> {
        if self.get(profile.as_ref()).is_some() {
            bail!("Profile '{}' already exists", profile.as_ref())
        }

        let path = noita_save_dir.as_ref().join("mod_config.xml");
        let mod_list = quick_xml::de::from_str::<Mods>(&fs::read_to_string(path)?)?;
        let path = self.write_profile(profile.as_ref(), &mod_list)?;
        self.insert(profile.as_ref().into(), path);

        Ok(mod_list)
    }
    pub fn get_profile(&self, profile: impl AsRef<str>) -> Result<Mods> {
        let path = self
            .get(profile.as_ref())
            .with_context(|| format!("Profile '{}' not found.", profile.as_ref()))?;

        Ok(quick_xml::de::from_str(&fs::read_to_string(path)?)?)
    }
    pub fn update_profile(&mut self, profile: impl AsRef<str>, mod_list: &Mods) -> Result<()> {
        if self.get(profile.as_ref()).is_none() {
            bail!("Profile '{}' doesn't exist", profile.as_ref())
        }
        self.write_profile(profile, mod_list)?;

        Ok(())
    }
    pub fn remove_profile(&mut self, profile: impl AsRef<str>) -> Result<()> {
        self.remove(profile.as_ref()).with_context(|| {
            format!(
                "Profile '{}' does not exist and cannot be removed",
                profile.as_ref()
            )
        })?;

        fs::remove_file(MOD_PROFILES_DIR.join(ModProfiles::get_profile_file_path(profile)))?;

        Ok(())
    }
    fn get_profile_file_path(profile: impl AsRef<str>) -> String {
        format!("{}.xml", profile.as_ref())
    }
    fn write_profile(&mut self, profile: impl AsRef<str>, mod_list: &Mods) -> Result<PathBuf> {
        fs::create_dir_all(MOD_PROFILES_DIR.as_path())?;
        let path = MOD_PROFILES_DIR.join(ModProfiles::get_profile_file_path(profile));
        fs::File::create(&path)?
            .write_fmt(format_args!("{}", quick_xml::se::to_string(mod_list)?))?;

        Ok(path)
    }
}

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
