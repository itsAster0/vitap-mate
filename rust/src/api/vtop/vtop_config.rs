use super::{session_manager::SessionManager, vtop_client::VtopClient};

#[derive(Debug, Clone)]
#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct BaseUrl(String);

impl BaseUrl {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BaseUrl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct VtopConfig {
    pub(crate) base_url: BaseUrl,
}

impl Default for VtopConfig {
    fn default() -> Self {
        Self {
            base_url: BaseUrl("https://vtop.vitap.ac.in".to_string()),
        }
    }
}

pub(crate) struct VtopClientBuilder {
    config: VtopConfig,
    session: SessionManager,
}

impl VtopClientBuilder {
    pub(crate) fn new() -> Self {
        Self {
            config: VtopConfig::default(),
            session: SessionManager::new(),
        }
    }

    pub(crate) fn build(self, username: String, password: String) -> VtopClient {
        VtopClient::with_config(self.config, self.session, username.to_uppercase(), password)
    }
}
