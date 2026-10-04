//! Shared code for our test harness.

use std::fs::File;
use std::path::Path;
use std::sync::Once;
use anyhow::{Context, Result};


static TRACING_INIT: Once = Once::new();

pub fn setup_logging() {
    use tracing_subscriber::{EnvFilter, fmt, fmt::time::LocalTime, prelude::*};
    use time::macros::format_description;

    TRACING_INIT.call_once(|| {
        let timer = LocalTime::new(format_description!("[hour]:[minute]:[second]"));
        let fmt_layer = fmt::layer()
            .compact()
            .with_timer(timer)
            .with_target(false);
        let filter_layer = EnvFilter::try_from_default_env()
        // The sqlx crate is used by the decrypt-cookies crate
            .or_else(|_| EnvFilter::try_new("info,reqwest=warn,hyper=warn,h2=warn,sqlx=warn"))
            .expect("initializing logging");
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();
    });
}


pub fn curl(url: &str, output: &Path) -> Result<()> {
    let mut response = reqwest::blocking::get(url)?;
    let mut out = File::create(output)
        .context("failed to create file")?;
    std::io::copy(&mut response, &mut out)
        .context("copying reqwest data to file")?;
    Ok(())
}
