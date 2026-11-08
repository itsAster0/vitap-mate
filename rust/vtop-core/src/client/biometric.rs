//! Campus biometric punch history.

use super::VtopClient;
use crate::error::VtopResult;
use crate::inputs::BiometricDate;
use crate::pages;
use crate::parser::biometric;
use crate::types::BiometricData;

impl VtopClient {
    pub async fn biometric_page(&self, date: &BiometricDate) -> VtopResult<String> {
        self.page("get_biometric_history", |s| pages::biometric(s, date))
            .await
    }

    pub async fn biometric_history(&self, date: &BiometricDate) -> VtopResult<BiometricData> {
        let page = self.biometric_page(date).await?;
        Ok(biometric::parse_biometric(&page, date.as_str()))
    }
}
