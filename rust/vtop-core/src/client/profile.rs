//! The student profile: personal, educational, family, proctor and hostel
//! details, with the photo; and the mentor (proctor) with their messages.

use super::VtopClient;
use crate::error::VtopResult;
use crate::parser::{mentor, profile};
use crate::types::{MentorData, StudentProfile};

impl VtopClient {
    /// A menu page that takes only the `verifyMenu` form.
    async fn menu_page(&self, context: &str, path: &str) -> VtopResult<String> {
        let auth = self.authorize(context).await?;
        let url = self.config.url(path);
        let body = format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            auth.registration_number, auth.csrf,
        );
        Self::log_request(&format!("{context}.send"), "POST", &url);
        self.send_authenticated(context, || self.http.post(&url).body(body.clone()))
            .await
    }

    pub async fn profile_page(&self) -> VtopResult<String> {
        self.menu_page("get_profile", "/vtop/studentsRecord/StudentProfileAllView")
            .await
    }

    pub async fn mentor_details_page(&self) -> VtopResult<String> {
        self.menu_page("get_mentor_details", "/vtop/proctor/viewProctorDetails")
            .await
    }

    pub async fn mentor_messages_page(&self) -> VtopResult<String> {
        self.menu_page(
            "get_mentor_messages",
            "/vtop/proctor/viewMessagesSendByProctor",
        )
        .await
    }

    pub async fn profile(&self) -> VtopResult<StudentProfile> {
        Ok(profile::parse_profile(&self.profile_page().await?))
    }

    pub async fn mentor(&self) -> VtopResult<MentorData> {
        let details = self.mentor_details_page().await?;
        let messages = self.mentor_messages_page().await?;
        Ok(mentor::parse_mentor(&details, &messages))
    }
}
