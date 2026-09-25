//! Campus biometric punch history.

use super::VtopClient;
use crate::error::VtopResult;
use crate::inputs::BiometricDate;
use crate::parser::biometric;
use crate::types::BiometricData;

impl VtopClient {
    pub async fn biometric_page(&self, date: &BiometricDate) -> VtopResult<String> {
        let auth = self.authorize("get_biometric_history").await?;
        let url = self.config.url("/vtop/getStudBioHistory");
        let form = [
            ("_csrf", auth.csrf.as_str()),
            ("fromDate", date.as_str()),
            ("authorizedID", auth.registration_number.as_str()),
        ];
        Self::log_request("get_biometric_history.send", "POST", &url);
        self.send_authenticated("get_biometric_history", || self.http.post(&url).form(&form))
            .await
    }

    pub async fn biometric_history(&self, date: &BiometricDate) -> VtopResult<BiometricData> {
        let page = self.biometric_page(date).await?;
        Ok(biometric::parse_biometric(&page, date.as_str()))
    }
}
