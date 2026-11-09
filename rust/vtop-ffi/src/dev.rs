//! Settings → Developer: put the app in odd states on purpose and tune the
//! client. Port of the Flutter app's `features/developer/debug_page.dart`
//! (Session, Saved data) and its tunables. Never logs cookies or CAs.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use serde_json::json;
use tracing::{info, warn};

use crate::exams::EXAMS;

/// The marks page (as in `marks.rs`).
const MARKS: &str = "marks.json";
use vtop_core::VtopConfig;
use vtop_core::types::{
    AttendanceData, ExamScheduleData, FullAttendanceData, MarksData, PersistedVtopSession,
    SemesterData, TimetableData,
};

use vtop_core::sync::RwLock;

use crate::{
    ACCOUNT, ATTENDANCE, ATTENDANCE_DETAIL, Account, Core, CoreError, PAGES, SELECTED, SEMESTERS,
    SESSION, Session, TIMETABLE, forwarding, records, traced, traced_sync,
};

/// The Flutter app's Cookie-Editor domain.
pub(crate) const COOKIE_DOMAIN: &str = "vtop.vitap.ac.in";

/// How the client is set up, read before [`Core::new`] (changes apply from the next launch).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DevConfig {
    /// A saved session older than this (minutes since its sign-in) is not restored; 0 = any age.
    pub session_reuse_minutes: u32,
    /// Upper bound for one VTOP request, seconds; 0 = vtop-core's default.
    pub request_timeout_s: u32,
}

static CONFIG: Mutex<Option<DevConfig>> = Mutex::new(None);

/// Sets [`DevConfig`]; call before the core is made. (Developer → Inspect Traffic is the app's
/// HTTP stack's: Rust sends nothing itself.)
#[uniffi::export]
pub fn dev_configure(config: DevConfig) {
    info!(
        session_reuse_minutes = config.session_reuse_minutes,
        request_timeout_s = config.request_timeout_s,
        "dev config"
    );
    *CONFIG.lock().unwrap_or_else(PoisonError::into_inner) = Some(config);
}

fn config() -> Option<DevConfig> {
    CONFIG
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// The client config every new VTOP client gets.
pub(crate) fn vtop_config() -> VtopConfig {
    let mut vtop = VtopConfig::default();
    if let Some(seconds) = config().map(|c| c.request_timeout_s).filter(|s| *s > 0) {
        vtop.request_timeout = Duration::from_secs(seconds.into());
    }
    vtop
}

/// Whether `saved` may be restored under the session-reuse limit.
pub(crate) fn reusable(saved: &PersistedVtopSession) -> bool {
    let minutes = config().map_or(0, |c| c.session_reuse_minutes);
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64);
    let ok = within(saved.saved_at_epoch_ms, now_ms, minutes);
    if !ok {
        info!(
            minutes,
            "saved session older than the reuse limit, not restored"
        );
    }
    ok
}

fn within(saved_at_ms: u64, now_ms: u64, minutes: u32) -> bool {
    minutes == 0 || now_ms.saturating_sub(saved_at_ms) <= u64::from(minutes) * 60_000
}

/// Which cached pages [`Core::dev_clear_cache`] removes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CacheScope {
    /// The selected semester's pages: attendance and its histories, timetable, calendar,
    /// marks and when each was seen, exams, grades.
    Semester,
    /// Every semester's pages, the profile and the grade history.
    All,
}

#[uniffi::export]
impl Core {
    /// Drops the VTOP session (`session.json` and the cookies, the web view's too) but keeps
    /// the account: the next fetch signs in again with the saved password. A pending OTP prompt
    /// goes too.
    pub async fn dev_clear_session(&self, flow: String) -> Result<(), CoreError> {
        traced("dev_clear_session", &flow, async {
            let slot = self.current()?;
            let mut session = slot.write().await;
            session.restart()?;
            remove(&self.at(SESSION));
            info!("session dropped; the next fetch signs in again");
            Ok(())
        })
        .await
    }

    /// Replaces the saved username and password in place, without asking VTOP (Developer → Set
    /// credentials without checking), and clears `rejected` so the next re-login tries them.
    /// Everything else stays: the session's cookies and reg no, the semester, every cached page.
    /// With no account yet, one is made with just these.
    pub async fn dev_set_account(
        &self,
        flow: String,
        username: String,
        password: String,
    ) -> Result<(), CoreError> {
        traced("dev_set_account", &flow, async {
            let account = Account {
                username: username.trim().to_uppercase(),
                password,
                rejected: false,
                registration_number: None,
            };
            let fresh = Session::new(account.clone())?;
            match self.current() {
                Ok(slot) => {
                    let mut session = slot.write().await;
                    // The session moves to a client holding the new credentials; cookies in
                    // the shared store stay there.
                    let mut saved = session.client.export_persisted_session(0);
                    if session.cookies.is_shared() {
                        saved.cookies = None;
                    }
                    self.restore(&fresh, Some(saved));
                    (session.client, session.cookies) = (fresh.client, fresh.cookies);
                    session.account = account.clone();
                }
                Err(_) => {
                    self.restore(&fresh, self.read::<PersistedVtopSession>(SESSION));
                    self.install(Some(Arc::new(RwLock::new(fresh))));
                }
            }
            self.write(ACCOUNT, &account);
            info!("credentials replaced without signing in");
            Ok(())
        })
        .await
    }

    /// Removes cached pages (see [`CacheScope`]); the account, session, semester list and pick
    /// stay, so open pages fetch again.
    pub fn dev_clear_cache(&self, flow: String, scope: CacheScope) -> Result<(), CoreError> {
        traced_sync("dev_clear_cache", &flow, || {
            match scope {
                CacheScope::All => self.forget(&[]),
                CacheScope::Semester => {
                    let Some(semester) = self.selected_semester() else {
                        info!("no semester picked: nothing to clear");
                        return Ok(());
                    };
                    remove_dir(&self.at(PAGES).join(semester));
                }
            }
            info!(?scope, "cache cleared");
            Ok(())
        })
    }

    /// Forgets the semester pick (pages stay cached per semester): the app asks for one again.
    pub fn dev_reset_semester(&self, flow: String) -> Result<(), CoreError> {
        traced_sync("dev_reset_semester", &flow, || {
            remove(&self.at(SELECTED));
            info!("semester pick forgotten");
            Ok(())
        })
    }

    /// Saves the selected semester's attendance with no courses, as before VTOP posts any (the
    /// Flutter app's Clear Attendance); a refresh restores it. False when none is saved.
    pub fn dev_clear_attendance(&self, flow: String) -> Result<bool, CoreError> {
        traced_sync("dev_clear_attendance", &flow, || {
            let Some(path) = self.page_path(ATTENDANCE) else {
                return Ok(false);
            };
            let Some(mut data) = self.read::<AttendanceData>(&path) else {
                return Ok(false);
            };
            data.records.clear();
            self.write(&path, &data);
            info!("attendance cleared");
            Ok(true)
        })
    }

    /// Removes the newest class from every saved course history of the selected semester, so
    /// it disagrees with the summary (the Flutter app's Drop Latest History Entry); a refresh
    /// restores it. Returns how many histories changed.
    pub fn dev_drop_latest_history(&self, flow: String) -> Result<u32, CoreError> {
        traced_sync("dev_drop_latest_history", &flow, || {
            let Some(dir) = self.page_path(ATTENDANCE_DETAIL) else {
                return Ok(0);
            };
            let mut dropped = 0;
            for entry in fs::read_dir(self.at(&dir)).into_iter().flatten().flatten() {
                let file = format!("{dir}/{}", entry.file_name().to_string_lossy());
                let Some(mut data) = self.read::<FullAttendanceData>(&file) else {
                    continue;
                };
                if drop_latest(&mut data) {
                    self.write(&file, &data);
                    dropped += 1;
                }
            }
            info!(dropped, "latest history entries dropped");
            Ok(dropped)
        })
    }

    /// Rolls the selected semester's saved pages back by one change each, without recording
    /// it, so the next fetch finds real differences for the change inbox and its
    /// notifications: a course's last class unposted (attendance), its newest score blanked
    /// (marks), one slot removed (timetable), one exam removed (exams). Returns how many pages
    /// changed; a refresh restores them all.
    pub fn dev_simulate_changes(&self, flow: String) -> Result<u32, CoreError> {
        traced_sync("dev_simulate_changes", &flow, || {
            let mut changed = 0;
            if let Some(path) = self.page_path(ATTENDANCE)
                && let Some(mut data) = self.read::<AttendanceData>(&path)
                && let Some(record) = data.records.iter_mut().find(|r| {
                    r.classes_attended.parse::<u32>().is_ok_and(|a| a > 0)
                        && r.total_classes.parse::<u32>().is_ok_and(|t| t > 1)
                })
            {
                let attended = record.classes_attended.parse::<u32>().unwrap_or(1) - 1;
                let total = record.total_classes.parse::<u32>().unwrap_or(2) - 1;
                record.classes_attended = attended.to_string();
                record.total_classes = total.to_string();
                record.attendance_percentage = (attended * 100).div_ceil(total).to_string();
                self.write_plain(&path, &data);
                changed += 1;
            }
            if let Some(path) = self.page_path(MARKS)
                && let Some(mut data) = self.read::<MarksData>(&path)
                && let Some(mark) = data
                    .records
                    .iter_mut()
                    .flat_map(|r| r.marks.iter_mut())
                    .rfind(|m| !m.scoredmark.trim().is_empty())
            {
                mark.scoredmark.clear();
                mark.weightagemark.clear();
                self.write_plain(&path, &data);
                changed += 1;
            }
            if let Some(path) = self.page_path(TIMETABLE)
                && let Some(mut data) = self.read::<TimetableData>(&path)
                && !data.slots.is_empty()
            {
                data.slots.remove(0);
                self.write_plain(&path, &data);
                changed += 1;
            }
            if let Some(path) = self.page_path(EXAMS)
                && let Some(mut data) = self.read::<ExamScheduleData>(&path)
                && let Some(exam) = data.exams.iter_mut().find(|e| !e.records.is_empty())
            {
                exam.records.remove(0);
                self.write_plain(&path, &data);
                changed += 1;
            }
            info!(changed, "pages rolled back for the change inbox");
            Ok(changed)
        })
    }

    /// The client's VTOP cookies as Cookie-Editor JSON (the Flutter app's Copy Saved Cookies),
    /// for a browser; None without cookies. Only how many is logged.
    pub async fn dev_cookie_editor_json(&self, flow: String) -> Result<Option<String>, CoreError> {
        traced("dev_cookie_editor_json", &flow, async {
            let slot = self.current()?;
            let header = slot.read().await.client.cookie_header().unwrap_or_default();
            let cookies = cookie_editor(&header);
            info!(cookies = cookies.len(), "cookies for cookie-editor");
            Ok((!cookies.is_empty()).then(|| {
                serde_json::to_string_pretty(&cookies).unwrap_or_else(|_| "[]".to_string())
            }))
        })
        .await
    }

    /// The edge new mail-forwarding addresses are made on (`forwarding_server`, else the
    /// default).
    pub fn dev_forwarding_server(&self, flow: String) -> String {
        traced_sync("dev_forwarding_server", &flow, || {
            Ok(self.forwarding_server())
        })
        .unwrap_or_default()
    }

    /// Points new forwarding setups at `url` (e.g. `http://127.0.0.1:8787`); None or blank
    /// goes back to the default. An existing address keeps its server.
    pub fn dev_set_forwarding_server(
        &self,
        flow: String,
        url: Option<String>,
    ) -> Result<(), CoreError> {
        traced_sync("dev_set_forwarding_server", &flow, || {
            let path = self.at(forwarding::SERVER_FILE);
            match url.as_deref().map(str::trim).filter(|url| !url.is_empty()) {
                Some(url) => {
                    if !(url.starts_with("http://") || url.starts_with("https://")) {
                        return Err(CoreError::Other {
                            reason: "the server must start with http:// or https://".to_string(),
                        });
                    }
                    fs::write(&path, format!("{url}\n"))?;
                    info!(server = url, "forwarding server set");
                }
                None => {
                    remove(&path);
                    info!("forwarding server back to the default");
                }
            }
            Ok(())
        })
    }
}

impl Core {
    /// The picked semester, if the cached list still has it.
    fn selected_semester(&self) -> Option<String> {
        self.pick(&self.read::<SemesterData>(SEMESTERS)?)
    }

    /// `file`'s path under the selected semester's pages.
    fn page_path(&self, file: &str) -> Option<String> {
        Some(format!("{PAGES}/{}/{file}", self.selected_semester()?))
    }
}

/// Removes the newest class (by its date; the first of a day's when tied) from `data`.
fn drop_latest(data: &mut FullAttendanceData) -> bool {
    let latest = data
        .records
        .iter()
        .enumerate()
        .filter_map(|(index, record)| Some((records::history_day(&record.date)?, index)))
        .max_by_key(|&(day, index)| (day, std::cmp::Reverse(index)));
    latest
        .map(|(_, index)| data.records.remove(index))
        .is_some()
}

/// A `Cookie` header as Cookie-Editor's import list, as the Flutter app's
/// `cookieEditorCookiesFromHeader`.
fn cookie_editor(header: &str) -> Vec<serde_json::Value> {
    header
        .split(';')
        .enumerate()
        .filter_map(|(index, part)| {
            let (name, value) = part.split_once('=')?;
            let (name, value) = (name.trim(), value.trim());
            (!name.is_empty() && !value.is_empty()).then(|| {
                json!({
                    "domain": COOKIE_DOMAIN, "hostOnly": true, "httpOnly": false,
                    "name": name, "path": "/", "sameSite": "unspecified", "secure": true,
                    "session": true, "storeId": "0", "value": value, "id": index + 1,
                })
            })
        })
        .collect()
}

fn remove(path: &Path) {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != ErrorKind::NotFound
    {
        warn!(%error, "not removed");
    }
}

fn remove_dir(path: &Path) {
    if let Err(error) = fs::remove_dir_all(path)
        && error.kind() != ErrorKind::NotFound
    {
        warn!(%error, "not removed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core(name: &str) -> (Core, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-dev-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let list = r#"{"semesters":[{"id":"AP1","name":"Fall"},{"id":"AP2","name":"Winter"}],"update_time":7}"#;
        fs::write(dir.join(SEMESTERS), list).unwrap();
        fs::write(dir.join(SELECTED), r#""AP1""#).unwrap();
        for semester in ["AP1", "AP2"] {
            fs::create_dir_all(dir.join(PAGES).join(semester).join(ATTENDANCE_DETAIL)).unwrap();
            fs::write(dir.join(PAGES).join(semester).join("marks_seen.json"), "{}").unwrap();
        }
        fs::write(dir.join(PAGES).join("profile.json"), "{}").unwrap();
        let core = Core {
            dir: dir.clone(),
            cache: dir.clone(),
            slot: Mutex::new(None),
        };
        (core, dir)
    }

    #[test]
    fn clear_cache_by_scope_keeps_list_and_pick() {
        let (core, dir) = core("cache");
        core.dev_clear_cache("f".into(), CacheScope::Semester)
            .unwrap();
        assert!(!dir.join(PAGES).join("AP1").exists());
        assert!(
            dir.join(PAGES).join("AP2").exists() && dir.join(PAGES).join("profile.json").exists()
        );
        core.dev_clear_cache("f".into(), CacheScope::All).unwrap();
        assert!(!dir.join(PAGES).exists());
        assert!(dir.join(SEMESTERS).exists() && dir.join(SELECTED).exists());
        core.dev_reset_semester("f".into()).unwrap();
        assert_eq!(core.cached_semesters("f".into()).unwrap().selected, None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn clear_session_keeps_the_account() {
        let (core, dir) = core("session");
        assert_eq!(
            core.dev_clear_session("f".into()).await,
            Err(CoreError::NotSignedIn)
        );
        let account = Account {
            username: "22BCE0001".into(),
            password: "pw-secret-1".into(),
            rejected: false,
            registration_number: None,
        };
        fs::write(
            dir.join(crate::ACCOUNT),
            serde_json::to_vec(&account).unwrap(),
        )
        .unwrap();
        fs::write(dir.join(SESSION), "{}").unwrap();
        let mut session = Session::new(account).unwrap();
        session.otp = Some("pending".into());
        core.install(Some(Arc::new(RwLock::new(session))));

        core.dev_clear_session("f".into()).await.unwrap();
        assert!(!dir.join(SESSION).exists());
        assert!(core.current().unwrap().read().await.otp.is_none());
        assert_eq!(
            core.signed_in_user("f".into()).as_deref(),
            Some("22BCE0001")
        );
        assert!(dir.join(PAGES).join("AP1").exists());
        assert_eq!(core.dev_cookie_editor_json("f".into()).await, Ok(None));
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn set_account_replaces_only_the_credentials() {
        let (core, dir) = core("account");
        assert_eq!(
            core.dev_set_account("f".into(), "x".into(), String::new())
                .await,
            Err(CoreError::InvalidCredentials)
        );
        assert!(!dir.join(ACCOUNT).exists());

        // No account yet: one is made with just these (and the saved session's cookies).
        let session = r#"{"username":"22BCE0001","saved_at_epoch_ms":1,"cookies":"JSESSIONID=abc","registration_number":"22BCE7777"}"#;
        fs::write(dir.join(SESSION), session).unwrap();
        core.dev_set_account("f".into(), " 22bce0001 ".into(), "pw-1".into())
            .await
            .unwrap();
        let saved: Account = core.read(ACCOUNT).unwrap();
        assert_eq!(
            (saved.username.as_str(), saved.password.as_str()),
            ("22BCE0001", "pw-1")
        );

        // A rejected account with a session: only the credentials change, under any username.
        let rejected = Account {
            rejected: true,
            ..saved
        };
        fs::write(dir.join(ACCOUNT), serde_json::to_vec(&rejected).unwrap()).unwrap();
        core.current().unwrap().write().await.account = rejected;
        fs::write(dir.join(forwarding::FILE), "{}").unwrap();
        core.dev_set_account("f".into(), "other".into(), "pw-2".into())
            .await
            .unwrap();
        let saved: Account = core.read(ACCOUNT).unwrap();
        assert_eq!(
            (
                saved.username.as_str(),
                saved.password.as_str(),
                saved.rejected
            ),
            ("OTHER", "pw-2", false)
        );
        assert!(!core.current().unwrap().read().await.account.rejected);
        assert_eq!(fs::read_to_string(dir.join(SESSION)).unwrap(), session);
        let cookies = core.dev_cookie_editor_json("f".into()).await.unwrap();
        assert!(cookies.is_some_and(|json| json.contains("JSESSIONID")));
        for kept in [SEMESTERS, SELECTED, forwarding::FILE] {
            assert!(dir.join(kept).exists(), "{kept}");
        }
        assert!(dir.join(PAGES).join("AP1").join("marks_seen.json").exists());
        assert!(dir.join(PAGES).join("profile.json").exists());
        assert_eq!(
            core.cached_semesters("f".into())
                .unwrap()
                .selected
                .as_deref(),
            Some("AP1")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saved_attendance_cleared_and_histories_trimmed() {
        let (core, dir) = core("saved");
        let page = dir.join(PAGES).join("AP1");
        fs::write(page.join(ATTENDANCE), r#"{"records":[{"serial":"1","category":"","course_name":"A","course_code":"A","course_type":"ETH","faculty_detail":"","classes_attended":"1","total_classes":"1","attendance_percentage":"100","attendence_fat_cat":"","debar_status":"","course_id":"x"}],"semester_id":"AP1","update_time":1}"#).unwrap();
        assert!(core.dev_clear_attendance("f".into()).unwrap());
        assert_eq!(core.cached_attendance("f".into()).unwrap().courses.len(), 0);

        let record = |serial: &str, date: &str| json!({"serial": serial, "date": date, "slot": "A1", "day_time": "", "status": "Present", "remark": ""});
        let history = json!({
            "records": [record("1", "01-Jul-2026"), record("2", "03-Jul-2026"), record("3", "02-Jul-2026")],
            "semester_id": "AP1", "update_time": 1, "course_id": "x", "course_type": "ETH",
        });
        fs::write(
            page.join(ATTENDANCE_DETAIL).join("x.json"),
            history.to_string(),
        )
        .unwrap();
        fs::write(page.join(ATTENDANCE_DETAIL).join("empty.json"), json!({"records": [], "semester_id": "AP1", "update_time": 1, "course_id": "y", "course_type": "ETH"}).to_string()).unwrap();
        assert_eq!(core.dev_drop_latest_history("f".into()).unwrap(), 1);
        let left: FullAttendanceData =
            serde_json::from_slice(&fs::read(page.join(ATTENDANCE_DETAIL).join("x.json")).unwrap())
                .unwrap();
        let serials: Vec<_> = left.records.iter().map(|r| r.serial.as_str()).collect();
        assert_eq!(serials, ["1", "3"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn forwarding_server_override() {
        let (core, dir) = core("server");
        let default = core.dev_forwarding_server("f".into());
        assert!(default.starts_with("https://"));
        assert!(
            core.dev_set_forwarding_server("f".into(), Some("ftp://x".into()))
                .is_err()
        );
        core.dev_set_forwarding_server("f".into(), Some(" http://127.0.0.1:8787 ".into()))
            .unwrap();
        assert_eq!(
            core.dev_forwarding_server("f".into()),
            "http://127.0.0.1:8787"
        );
        core.dev_set_forwarding_server("f".into(), None).unwrap();
        assert_eq!(core.dev_forwarding_server("f".into()), default);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cookie_editor_shape_and_reuse_limit() {
        let cookies = cookie_editor("JSESSIONID=abc; =x; SERVERID=s1;loose");
        assert_eq!(cookies.len(), 2);
        assert_eq!(
            (cookies[0]["name"].as_str(), cookies[0]["id"].as_u64()),
            (Some("JSESSIONID"), Some(1))
        );
        assert_eq!(
            (cookies[1]["value"].as_str(), cookies[1]["id"].as_u64()),
            (Some("s1"), Some(3))
        );
        assert!(within(0, u64::MAX, 0));
        assert!(within(0, 30 * 60_000, 30));
        assert!(!within(0, 30 * 60_000 + 1, 30));
    }

    #[test]
    fn saved_account_reports_the_rejected_flag() {
        let (core, _dir) = core("saved_account");
        assert!(core.saved_account("f".into()).is_none());
        let account = Account {
            username: "22BCE0001".into(),
            password: "pw-1".into(),
            rejected: true,
            registration_number: None,
        };
        core.write(ACCOUNT, &account);
        // No Debug on SavedAccount, so a failing test never prints the password.
        let saved = core.saved_account("f".into()).unwrap();
        assert!(saved.username == "22BCE0001" && saved.password == "pw-1" && saved.rejected);
    }

    #[test]
    fn accepted_password_is_kept_only_for_the_saved_account() {
        let (core, _dir) = core("keep_password");
        let saved = Account {
            username: "22BCE0001".into(),
            password: "old".into(),
            rejected: true,
            registration_number: None,
        };
        core.write(ACCOUNT, &saved);

        // Another username: the saved account stays as it is.
        core.keep_accepted_password(&Account {
            username: "22BCE0002".into(),
            password: "other".into(),
            rejected: false,
            registration_number: None,
        });
        let kept: Account = core.read(ACCOUNT).unwrap();
        assert!(kept.username == "22BCE0001" && kept.password == "old" && kept.rejected);

        // The same account with the new password VTOP accepted: saved, no longer rejected.
        core.keep_accepted_password(&Account {
            username: "22BCE0001".into(),
            password: "new".into(),
            rejected: false,
            registration_number: None,
        });
        let kept: Account = core.read(ACCOUNT).unwrap();
        assert!(kept.password == "new" && !kept.rejected);
    }
}
