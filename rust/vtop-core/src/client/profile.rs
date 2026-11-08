//! The student profile: personal, educational, family, proctor and hostel
//! details, with the photo; and the mentor (proctor) with their messages.

use super::VtopClient;
use crate::error::VtopResult;
use crate::pages;
use crate::parser::{mentor, profile};
use crate::types::{MentorData, StudentProfile};

impl VtopClient {
    /// A menu page that takes only the `verifyMenu` form.
    pub async fn profile_page(&self) -> VtopResult<String> {
        self.page("get_profile", pages::profile).await
    }

    pub async fn mentor_details_page(&self) -> VtopResult<String> {
        self.page("get_mentor_details", pages::mentor_details).await
    }

    pub async fn mentor_messages_page(&self) -> VtopResult<String> {
        self.page("get_mentor_messages", pages::mentor_messages)
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
