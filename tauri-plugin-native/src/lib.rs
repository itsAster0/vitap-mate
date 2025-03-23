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
use desktop::Native;
#[cfg(mobile)]
use mobile::Native;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the native APIs.
pub trait NativeExt<R: Runtime> {
  fn native(&self) -> &Native<R>;
}

impl<R: Runtime, T: Manager<R>> crate::NativeExt<R> for T {
  fn native(&self) -> &Native<R> {
    self.state::<Native<R>>().inner()
  }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("native")
    .invoke_handler(tauri::generate_handler![commands::ping])
    .setup(|app, api| {
      #[cfg(mobile)]
      let native = mobile::init(app, api)?;
      #[cfg(desktop)]
      let native = desktop::init(app, api)?;
      app.manage(native);
      Ok(())
    })
    .build()
}
