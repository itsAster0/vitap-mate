//! Solves the login CAPTCHA with the embedded model. There is no remote
//! solver: the image never leaves the device (or server) doing the login.

use log::Level;

use super::VtopClient;
use crate::error::{VtopError, VtopResult};

impl VtopClient {
    pub(super) async fn solve_captcha(&self, captcha_data: String) -> VtopResult<String> {
        // Image decoding and the dense layer are CPU work; keep them off the
        // async executor.
        let solved =
            tokio::task::spawn_blocking(move || crate::captcha::solve_data_url(&captcha_data))
                .await
                .map_err(|error| {
                    VtopError::ConfigurationError(format!("CAPTCHA solver task failed: {error}"))
                })?;
        solved.map_err(|error| {
            self.auth_log(Level::Error, "captcha", &error);
            VtopError::ConfigurationError(error)
        })
    }
}
