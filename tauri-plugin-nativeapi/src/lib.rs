use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Nativeapi;
#[cfg(mobile)]
use mobile::Nativeapi;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the nativeapi APIs.
pub trait NativeapiExt<R: Runtime> {
  fn nativeapi(&self) -> &Nativeapi<R>;
}

impl<R: Runtime, T: Manager<R>> crate::NativeapiExt<R> for T {
  fn nativeapi(&self) -> &Nativeapi<R> {
    self.state::<Nativeapi<R>>().inner()
  }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("nativeapi")
    .invoke_handler(tauri::generate_handler![commands::ping])
    .setup(|app, api| {
      #[cfg(mobile)]
      let nativeapi = mobile::init(app, api)?;
      #[cfg(desktop)]
      let nativeapi = desktop::init(app, api)?;
      app.manage(nativeapi);
      Ok(())
    })
    .build()
}
