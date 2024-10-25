use color_eyre::eyre::Result;
use fs_err as fs;
use itertools::Itertools;
use std::{
    io::{self, BufWriter},
    path::Path,
};
use tracing::warn;

use walkdir::WalkDir;

pub struct RotatingWriter {
    sink: BufWriter<fs::File>,
}

impl RotatingWriter {
    pub fn new(mut max: usize, dir: impl AsRef<Path>, prefix: impl AsRef<str>) -> Result<Self> {
        fs::create_dir_all(dir.as_ref())?;

        if max < 1 {
            max = 1
        }

        let logs = WalkDir::new(dir.as_ref())
            .max_depth(1)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| {
                e.file_type().is_file()
                    && e.file_name().to_string_lossy().starts_with(prefix.as_ref())
            })
            .flat_map(|it| it.ok())
            .sorted_by(|a, b| Ord::cmp(&a.file_name(), &b.file_name()))
            .collect_vec();

        if logs.len() >= max {
            let extra_logs = logs.len() - max;
            for i in 0..extra_logs + 1 {
                _ = fs::remove_file(
                    logs.get(i)
                        .expect("loop only iterates over valid indices")
                        .path(),
                )
                .inspect_err(|e| warn!(%e));
            }
        }

        let time = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S");

        Ok(Self {
            sink: BufWriter::new(fs::File::create(
                dir.as_ref().join(format!("{}.{time}", prefix.as_ref())),
            )?),
        })
    }
}

impl io::Write for RotatingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.sink.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
