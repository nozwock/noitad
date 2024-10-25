use color_eyre::eyre::Result;
use noitad_lib::{defines::APP_CONFIG_PATH, log::RotatingWriter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> Result<()> {
    let (non_blocking, _guard) = tracing_appender::non_blocking(RotatingWriter::new(
        3,
        APP_CONFIG_PATH.join("logs"),
        "noitd.log",
    )?);
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(non_blocking))
        .with(EnvFilter::from_default_env())
        .init();

    println!("Hello, world!");

    Ok(())
}
