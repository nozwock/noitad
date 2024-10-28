use std::path::Path;

use color_eyre::eyre::{bail, Result};

pub trait PathExt: AsRef<Path>
where
    Self: Sized,
{
    fn try_is_dir(self) -> Result<Self> {
        match self.as_ref().is_dir() {
            true => Ok(self),
            false => bail!("Path {:?} isn't a directory", self.as_ref()),
        }
    }
    fn try_is_file(self) -> Result<Self> {
        match self.as_ref().is_dir() {
            true => Ok(self),
            false => bail!("Path {:?} isn't a file", self.as_ref()),
        }
    }
}

impl<T: AsRef<Path>> PathExt for T {}
