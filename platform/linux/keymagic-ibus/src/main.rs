//! `IBus` engine process: serves `KeyMagic` over the `IBus` D-Bus protocol.

mod bus;
mod context;
mod factory;
mod input;
mod text;

use keymagic_host::{KeyboardStore, Paths};
use std::process::ExitCode;
use std::sync::Arc;
use zbus::connection;

/// Well-known name the component file declares for this engine process.
const BUS_NAME: &str = "org.freedesktop.IBus.KeyMagic";

/// Object path the `IBus` daemon calls to create contexts.
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

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
