//! VTOP's semesters and the one pages show ([`SemesterState`]). Rust keeps the pick and a cache
//! per semester; the app mirrors the state ([`crate::AppEvents::on_semesters`]). Nothing is
//! guessed: with no pick the app shows the picker, as the Flutter app's onboarding.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::page_watch::PageActivity;
use crate::pages::records::Semesters;
use crate::problem::Problem;
use crate::{Core, CoreError};

/// The semester list as the picker and Settings show it.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum SemesterState {
    /// None cached (or not read yet), and none being fetched.
    None,
    /// None cached; fetching it.
    Loading,
    /// None cached, and the fetch failed.
    Failed { problem: Problem },
    /// The list, and what its refresh is doing.
    Ready {
        semesters: Semesters,
        activity: PageActivity,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum SemesterEvent {
    /// The cached list (None: none, e.g. after sign-out).
    CacheRead(Option<Semesters>),
    RefreshStarted,
    RefreshSucceeded(Semesters),
    RefreshFailed(Problem),
    /// The refresh stopped without an answer (its screen went).
    RefreshCancelled,
}

const REFRESHING: PageActivity = PageActivity::Refreshing { waiting: None };

pub(crate) fn next(state: SemesterState, event: SemesterEvent) -> SemesterState {
    use SemesterEvent as E;
    use SemesterState as S;
    match (state, event) {
        (state, E::CacheRead(Some(semesters))) => S::Ready {
            semesters,
            activity: match state {
                S::Ready { activity, .. } => activity,
                S::Loading => REFRESHING,
                S::Failed { problem } => PageActivity::Stale { problem },
                S::None => PageActivity::Idle,
            },
        },
        // The list went (sign-out); a refresh may still bring one.
        (S::Ready { activity, .. }, E::CacheRead(None)) => match activity {
            PageActivity::Refreshing { .. } => S::Loading,
            _ => S::None,
        },
        (state, E::CacheRead(None)) => state,
        (S::Ready { semesters, .. }, E::RefreshStarted) => S::Ready {
            semesters,
            activity: REFRESHING,
        },
        (_, E::RefreshStarted) => S::Loading,
        (_, E::RefreshSucceeded(semesters)) => S::Ready {
            semesters,
            activity: PageActivity::Idle,
        },
        (S::Ready { semesters, .. }, E::RefreshFailed(problem)) => S::Ready {
            semesters,
            activity: PageActivity::Stale { problem },
        },
        (_, E::RefreshFailed(problem)) => S::Failed { problem },
        (
            S::Ready {
                semesters,
                activity,
            },
            E::RefreshCancelled,
        ) => S::Ready {
            semesters,
            activity: activity.settled(),
        },
        (S::Loading, E::RefreshCancelled) => S::None,
        (state @ (S::None | S::Failed { .. }), E::RefreshCancelled) => state,
    }
}

/// The state, changed by one event at a time.
pub(crate) struct Choice {
    state: Mutex<SemesterState>,
}

impl Default for Choice {
    fn default() -> Self {
        Self {
            state: Mutex::new(SemesterState::None),
        }
    }
}

impl Core {
    /// Applies `event` and tells the app, under the lock so pushes keep their order.
    fn on_semesters(&self, event: SemesterEvent) -> SemesterState {
        let mut state = self
            .sync
            .choice
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        *state = next(std::mem::replace(&mut *state, SemesterState::None), event);
        let now = state.clone();
        self.events
            .tell("on_semesters", |events| events.on_semesters(now.clone()));
        now
    }

    /// The semester pages show: the pick, if the cached list still has it.
    pub(crate) fn selected_semester(&self) -> Option<String> {
        self.pick(&self.read_kept(crate::SEMESTERS)?)
    }
}

/// Ends a refresh dropped before its answer as cancelled.
struct Refreshing<'a> {
    core: &'a Core,
    armed: bool,
}

impl Drop for Refreshing<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.core.on_semesters(SemesterEvent::RefreshCancelled);
        }
    }
}

#[uniffi::export]
impl Core {
    /// The cached list and pick (a page fetch caches the list too), without the network; the
    /// state after it (the app has been told it already).
    pub fn semester_read_cache(&self, flow: String) -> SemesterState {
        let cached = self.cached_semesters(flow);
        self.on_semesters(SemesterEvent::CacheRead(cached))
    }

    /// Fetches the list from VTOP; a failure is kept in the state.
    pub async fn semester_refresh(&self, flow: String, trigger: String) {
        self.on_semesters(SemesterEvent::RefreshStarted);
        let mut guard = Refreshing {
            core: self,
            armed: true,
        };
        let fetched = self
            .schedule
            .activity
            .track(
                "Loading semesters",
                &flow,
                self.fetch_semesters(flow.clone()),
            )
            .await;
        guard.armed = false;
        match fetched {
            Ok(fresh) => {
                let count = fresh.items.len().to_string();
                self.on_semesters(SemesterEvent::RefreshSucceeded(fresh));
                log_ui(
                    "info",
                    &flow,
                    "fresh",
                    [("trigger", trigger), ("count", count)],
                );
            }
            Err(error) => {
                let problem = Problem::of(&error, &flow, false);
                let kind = format!("{:?}", problem.kind);
                self.on_semesters(SemesterEvent::RefreshFailed(problem));
                log_ui(
                    "warn",
                    &flow,
                    "refresh failed",
                    [("trigger", trigger), ("kind", kind)],
                );
            }
        }
    }

    /// Shows `id`'s pages from now on; false when it was the pick already.
    pub fn semester_pick(&self, flow: String, id: String) -> Result<bool, CoreError> {
        if self.selected_semester().as_deref() == Some(id.as_str()) {
            return Ok(false);
        }
        self.select_semester(flow.clone(), id.clone())?;
        self.semester_read_cache(flow.clone());
        log_ui("info", &flow, "selected", [("semester", id)]);
        Ok(true)
    }
}

/// A `ui.semesters` line, as Kotlin wrote it.
fn log_ui<const N: usize>(level: &str, flow: &str, msg: &str, fields: [(&str, String); N]) {
    let fields: HashMap<String, String> = fields
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
    crate::obs::log_event(
        level.into(),
        "ui.semesters".into(),
        Some(flow.into()),
        msg.into(),
        fields,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::records::Semester;

    fn offline() -> Problem {
        Problem::of(&CoreError::Offline, "f1", false)
    }

    fn list() -> Semesters {
        Semesters {
            items: vec![Semester::new("AP1".into(), "Fall 2026".into())],
            selected: Some("AP1".into()),
            updated_at: 10,
        }
    }

    fn newer() -> Semesters {
        Semesters {
            updated_at: 20,
            ..list()
        }
    }

    fn ready(semesters: Semesters, activity: PageActivity) -> SemesterState {
        SemesterState::Ready {
            semesters,
            activity,
        }
    }

    fn after(state: SemesterState, events: Vec<SemesterEvent>) -> SemesterState {
        events.into_iter().fold(state, next)
    }

    use SemesterEvent as E;
    use SemesterState as S;

    #[test]
    fn first_fetch_loads_then_settles() {
        assert_eq!(S::Loading, next(S::None, E::RefreshStarted));
        assert_eq!(
            ready(list(), PageActivity::Idle),
            after(
                S::None,
                vec![E::RefreshStarted, E::RefreshSucceeded(list())]
            )
        );
        assert_eq!(
            S::Failed { problem: offline() },
            after(
                S::None,
                vec![E::RefreshStarted, E::RefreshFailed(offline())]
            )
        );
        assert_eq!(
            S::None,
            after(S::None, vec![E::RefreshStarted, E::RefreshCancelled])
        );
    }

    #[test]
    fn a_cached_list_stays_through_a_failed_refresh() {
        let ready_now = next(S::None, E::CacheRead(Some(list())));
        assert_eq!(ready(list(), PageActivity::Idle), ready_now);
        assert_eq!(
            ready(list(), REFRESHING),
            next(ready_now.clone(), E::RefreshStarted)
        );
        let stale = after(
            ready_now,
            vec![E::RefreshStarted, E::RefreshFailed(offline())],
        );
        assert_eq!(
            ready(list(), PageActivity::Stale { problem: offline() }),
            stale
        );
        assert_eq!(
            ready(newer(), PageActivity::Stale { problem: offline() }),
            next(stale.clone(), E::CacheRead(Some(newer())))
        );
        assert_eq!(
            ready(newer(), PageActivity::Idle),
            after(stale, vec![E::RefreshStarted, E::RefreshSucceeded(newer())])
        );
    }

    #[test]
    fn a_cache_read_mid_refresh_keeps_its_activity() {
        assert_eq!(
            ready(list(), REFRESHING),
            next(S::Loading, E::CacheRead(Some(list())))
        );
        assert_eq!(
            ready(list(), PageActivity::Stale { problem: offline() }),
            next(S::Failed { problem: offline() }, E::CacheRead(Some(list())))
        );
    }

    #[test]
    fn the_list_going_resets() {
        assert_eq!(
            S::None,
            next(ready(list(), PageActivity::Idle), E::CacheRead(None))
        );
        assert_eq!(
            S::Loading,
            next(ready(list(), REFRESHING), E::CacheRead(None))
        );
        assert_eq!(
            S::Failed { problem: offline() },
            next(S::Failed { problem: offline() }, E::CacheRead(None))
        );
    }
}
