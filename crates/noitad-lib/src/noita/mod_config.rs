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
