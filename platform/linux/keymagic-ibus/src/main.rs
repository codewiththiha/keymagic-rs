//! `IBus` engine process: serves `KeyMagic` over the `IBus` D-Bus protocol.

#[cfg(target_os = "linux")]
mod bus;
#[cfg(target_os = "linux")]
mod context;
#[cfg(target_os = "linux")]
mod factory;
#[cfg(target_os = "linux")]
mod input;
#[cfg(target_os = "linux")]
mod menu;
#[cfg(target_os = "linux")]
mod text;

#[cfg(target_os = "linux")]
use keymagic_host::{KeyboardStore, Paths};
#[cfg(target_os = "linux")]
use std::sync::Arc;
#[cfg(target_os = "linux")]
use zbus::connection;

use std::process::ExitCode;

/// Well-known name the component file declares for this engine process.
#[cfg(target_os = "linux")]
const BUS_NAME: &str = "org.freedesktop.IBus.KeyMagic";

/// Object path the `IBus` daemon calls to create contexts.
#[cfg(target_os = "linux")]
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

#[cfg(target_os = "linux")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> ExitCode {
    eprintln!("error: keymagic-ibus runs on Linux only");
    ExitCode::FAILURE
}

#[cfg(target_os = "linux")]
async fn run() -> Result<(), String> {
    let paths = Paths::for_user().ok_or("no user folders for this account")?;
    let store = Arc::new(KeyboardStore::new(&paths));
    let address = bus::address()?;
    let factory = factory::Factory::new(store, paths.config_file());
    let _connection = connection::Builder::address(address.as_str())
        .name(BUS_NAME)
        .serve_at(FACTORY_PATH, factory)
        .build()
        .await
        .map_err(|error| error.to_string())?;
    std::future::pending::<()>().await;
    Ok(())
}
