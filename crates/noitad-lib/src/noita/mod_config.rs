use std::{io::Write, path::Path};

use color_eyre::eyre::Result;
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
    pub fn overwrite_noita_mod_list(
        mod_list: &Mods,
        noita_save_dir: impl AsRef<Path>,
    ) -> Result<()> {
        fs::File::create(noita_save_dir.as_ref().join("mod_config.xml"))?
            .write_fmt(format_args!("{}", quick_xml::se::to_string(mod_list)?))?;

        Ok(())
    }
}
