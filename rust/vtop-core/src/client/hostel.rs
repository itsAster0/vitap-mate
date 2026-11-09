//! Hostel outings: the lists, apply and cancel, and the pass, run as the
//! `vtop-proto` outing machines. Apply and cancel are sent once, never
//! retried.

use super::VtopClient;
use crate::error::VtopResult;
use crate::inputs::OutingPassId;
use crate::machine::OnePost;
use crate::outing::{self, Application, Apply, Cancel};
use crate::parser::hostel;
use crate::types::{GeneralOutingData, OutingApplyResult, OutingCancelResult, WeekendOutingData};

pub use crate::outing::{
    check_general_outing, check_weekend_outing, GeneralOutingApplication, OutingKind,
    WeekendOutingApplication,
};

impl VtopClient {
    pub async fn general_outing_page(&self) -> VtopResult<String> {
        self.page("get_general_outing", |s| {
            outing::list(s, OutingKind::General)
        })
        .await
    }

    pub async fn general_outing(&self) -> VtopResult<GeneralOutingData> {
        Ok(hostel::parse_general_outing(
            &self.general_outing_page().await?,
        ))
    }

    pub async fn weekend_outing_page(&self) -> VtopResult<String> {
        self.page("get_weekend_outing", |s| {
            outing::list(s, OutingKind::Weekend)
        })
        .await
    }

    pub async fn weekend_outing(&self) -> VtopResult<WeekendOutingData> {
        Ok(hostel::parse_weekend_outing(
            &self.weekend_outing_page().await?,
        ))
    }

    pub async fn apply_general_outing(
        &self,
        application: &GeneralOutingApplication,
    ) -> VtopResult<OutingApplyResult> {
        let signed = self.signed("apply_general_outing").await?;
        let machine = Apply::new(signed, Application::General(application.clone()));
        self.run("apply_general_outing", machine).await
    }

    pub async fn apply_weekend_outing(
        &self,
        application: &WeekendOutingApplication,
    ) -> VtopResult<OutingApplyResult> {
        let signed = self.signed("apply_weekend_outing").await?;
        let machine = Apply::new(signed, Application::Weekend(application.clone()));
        self.run("apply_weekend_outing", machine).await
    }

    pub async fn cancel_general_outing(&self, id: &OutingPassId) -> VtopResult<OutingCancelResult> {
        let signed = self.signed("cancel_general_outing").await?;
        let machine = Cancel::new(signed, OutingKind::General, id.clone());
        self.run("cancel_general_outing", machine).await
    }

    pub async fn cancel_weekend_outing(&self, id: &OutingPassId) -> VtopResult<OutingCancelResult> {
        let signed = self.signed("cancel_weekend_outing").await?;
        let machine = Cancel::new(signed, OutingKind::Weekend, id.clone());
        self.run("cancel_weekend_outing", machine).await
    }

    pub async fn outing_pass(&self, kind: OutingKind, id: &OutingPassId) -> VtopResult<Vec<u8>> {
        let signed = self.signed("get_outing_pass").await?;
        let machine = OnePost::reading(outing::pass(&signed, kind, id), outing::read_pass);
        self.run("get_outing_pass", machine).await
    }

    /// [`Self::outing_pass`] written to the file `to` (the app's transport
    /// writes it there itself); checked as [`outing::read_pass`] does, from
    /// the head and the first bytes (`%PDF`). Returns its size; nothing stays
    /// at `to` when VTOP did not send the pass.
    pub async fn save_outing_pass(
        &self,
        kind: OutingKind,
        id: &OutingPassId,
        to: &std::path::Path,
    ) -> VtopResult<u64> {
        let context = "get_outing_pass";
        let signed = self.signed(context).await?;
        let (head, size) = self
            .download(context, outing::pass(&signed, kind, id), to)
            .await?;
        if let Err(error) = outing::read_pass(&head) {
            let _ = std::fs::remove_file(to);
            if matches!(error, crate::error::VtopError::SessionExpired) {
                self.mark_session_expired(context, &format!("VTOP answered: {error}"));
            } else {
                log::warn!(target: super::NETWORK, "{context}: {error}");
            }
            return Err(error);
        }
        Ok(size)
    }
}
