use std::{path::PathBuf, sync::LazyLock};

pub const NOITA_STEAM_ID: u32 = 881100;

pub const APP_DIR: &str = "io.github.nozwock.noitd";

pub static APP_CONFIG_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    directories::BaseDirs::new()
        .map(|it| it.config_local_dir().join(APP_DIR))
        .unwrap_or_default()
});
