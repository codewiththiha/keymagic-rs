//! Serves `org.freedesktop.IBus.Factory`, which creates one context per field.

use crate::context::Context;
use keymagic_host::KeyboardStore;
use std::path::PathBuf;
use std::sync::Arc;
use zbus::ObjectPath;

/// Object path prefix for engine contexts.
const ENGINE_PREFIX: &str = "/org/freedesktop/IBus/Engine/";

/// Factory object the `IBus` daemon calls to create contexts.
pub struct Factory {
    store: Arc<KeyboardStore>,
    config_path: PathBuf,
    created: u32,
}

impl Factory {
    /// Creates a factory that loads keyboards from `store`.
    #[must_use]
    pub const fn new(store: Arc<KeyboardStore>, config_path: PathBuf) -> Self {
        Self {
            store,
            config_path,
            created: 0,
        }
    }
}

#[zbus::interface(name = "org.freedesktop.IBus.Factory")]
impl Factory {
    /// Creates a context for `name` and serves it at a new object path.
    async fn create_engine(
        &mut self,
        name: &str,
        #[zbus(object_server)] server: &zbus::ObjectServer,
    ) -> zbus::fdo::Result<ObjectPath<'static>> {
        self.created += 1;
        let path = ObjectPath::try_from(format!("{ENGINE_PREFIX}{}", self.created))
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
        let context = Context::load(
            Arc::clone(&self.store),
            self.config_path.clone(),
            name,
            path.clone(),
        )
        .map_err(zbus::fdo::Error::Failed)?;
        server
            .at(path.clone(), context)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
        Ok(path)
    }
}
