//! Hostel outings: apply and cancel, and the pass, run as the
//! `vtop-proto` outing machines. Apply and cancel are sent once, never
//! retried.

use super::{FileLink, VtopClient};
use crate::error::VtopResult;
use crate::inputs::OutingPassId;
use crate::machine::OnePost;
use crate::outing::{self, Application, Apply, Cancel};
use crate::types::{OutingApplyResult, OutingCancelResult};

pub use crate::outing::{
    GeneralOutingApplication, OutingKind, WeekendOutingApplication, check_general_outing,
    check_weekend_outing,
};

impl VtopClient {
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

    /// [`Self::outing_pass`] for a downloader outside this client (Android's
    /// DownloadManager), as [`Self::course_page_file_link`].
    pub async fn outing_pass_link(
        &self,
        kind: OutingKind,
        id: &OutingPassId,
    ) -> VtopResult<FileLink> {
        let signed = self.signed("get_outing_pass").await?;
        self.file_link(outing::pass(&signed, kind, id)).await
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
            #[expect(
                clippy::let_underscore_must_use,
                reason = "the error is VTOP's answer; a file left behind is only a stray download"
            )]
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
