use std::{io::Write, path::Path};

use color_eyre::eyre::{self, Result};
use fs_err as fs;
use serde::{Deserialize, Serialize, Serializer};

fn serialize_bool_as_number<S: Serializer>(value: &bool, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(if *value { "1" } else { "0" })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mods {
    #[serde(rename = "Mod")]
    pub mods: Vec<Mod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    #[serde(rename = "@enabled")]
    #[serde(serialize_with = "serialize_bool_as_number")]
    pub enabled: bool,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@settings_fold_open")]
    #[serde(serialize_with = "serialize_bool_as_number")]
    pub settings_fold_open: bool,
    #[serde(rename = "@workshop_item_id")]
    pub workshop_item_id: usize,
}

impl Mods {
    pub fn from_noita(noita_save_dir: impl AsRef<Path>) -> Result<Self> {
        quick_xml::de::from_str(&fs::read_to_string(
            noita_save_dir.as_ref().join("mod_config.xml"),
        )?)
        .map_err(eyre::Report::msg)
    }
    pub fn sync_with_noita(mod_list: &mut Mods, noita_save_dir: impl AsRef<Path>) -> Result<()> {
        let noita_mod_list = Self::from_noita(noita_save_dir.as_ref())?;

        let mut new_mods = vec![];

        let len = mod_list.mods.len();
        for i in 0..len {
            for mod_ in noita_mod_list.mods.iter() {
                if mod_list.mods[i].name == mod_.name {
                    mod_list.mods[i].enabled = mod_.enabled;
                    mod_list.mods[i].settings_fold_open = mod_.settings_fold_open;
                    mod_list.mods[i].workshop_item_id = mod_.workshop_item_id;
                } else {
                    new_mods.push(mod_);
                }
            }
        }

        for mod_ in new_mods.into_iter() {
            mod_list.mods.push(mod_.to_owned());
        }

        Ok(())
    }
    pub fn overwrite_noita_mod_list(
        mod_list: &Mods,
        noita_save_dir: impl AsRef<Path>,
    ) -> Result<()> {
        fs::File::create(noita_save_dir.as_ref().join("mod_config.xml"))?
            .write_fmt(format_args!("{}", quick_xml::se::to_string(mod_list)?))?;

        Ok(())
    }
}
