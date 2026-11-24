//! The route policy with no I/O and no clock, as vtop-proto's page machines are: [`RoutePlan`]
//! says which route to try and keeps the skips, and [`ServerFetch`] steps a server page through
//! consent, the fetch and one sign-in again. The caller passes `now` (unix seconds) and runs the
//! steps.

use std::collections::BTreeMap;

use super::{Route, RouteStatus, SkippedRoute};

/// How long a failed route is skipped before it is tried again.
pub(super) const SKIP_S: i64 = 5 * 60;

/// How a try over a route went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Outcome {
    /// VTOP's answer came by it.
    Used,
    /// It failed, for this reason.
    Failed(String),
}

/// The chosen route and the ones skipped after they failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoutePlan {
    pub(super) chosen: Route,
    skipped: BTreeMap<Route, (i64, String)>,
    last: Option<(Route, i64)>,
}

impl RoutePlan {
    pub(super) const fn new() -> Self {
        Self {
            chosen: Route::Direct,
            skipped: BTreeMap::new(),
            last: None,
        }
    }

    /// Whether `route` should be tried now: the chosen one covers it (UDP falls back to the
    /// HTTPS server route) and it isn't skipped.
    pub(super) fn active(&self, route: Route, now: i64) -> bool {
        let chosen = self.chosen == route || (self.chosen == Route::Udp && route == Route::Server);
        chosen
            && self
                .skipped
                .get(&route)
                .is_none_or(|(until, _)| *until <= now)
    }

    /// The server route a page goes by first (UDP, then HTTPS), or None for the phone.
    pub(super) fn next(&self, now: i64) -> Option<Route> {
        [Route::Udp, Route::Server]
            .into_iter()
            .find(|route| self.active(*route, now))
    }

    pub(super) fn on_result(&mut self, route: Route, outcome: Outcome, now: i64) {
        match outcome {
            Outcome::Used => {
                self.skipped.remove(&route);
                self.last = Some((route, now));
            }
            Outcome::Failed(reason) => {
                self.skipped.insert(route, (now + SKIP_S, reason));
            }
        }
    }

    /// `route` chosen, with earlier failures forgotten so it is tried straight away.
    pub(super) fn choose(&mut self, route: Route) {
        self.chosen = route;
        self.skipped.clear();
    }

    /// Back to Direct with nothing skipped or used.
    pub(super) fn forget(&mut self) {
        self.choose(Route::Direct);
        self.last = None;
    }

    pub(super) fn status(&self, now: i64) -> RouteStatus {
        let mut skipped: Vec<SkippedRoute> = self
            .skipped
            .iter()
            .filter(|(_, (until, _))| *until > now)
            .map(|(route, (until, reason))| SkippedRoute {
                route: *route,
                until: *until,
                reason: reason.clone(),
            })
            .collect();
        skipped.sort_by_key(|skip| skip.until);
        RouteStatus {
            chosen: self.chosen,
            last_used: self.last.map(|(route, _)| route),
            last_used_at: self.last.map(|(_, at)| at),
            skipped,
        }
    }
}

/// The edge's answer as [`ServerError`]: the session ran out, or the server part failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ServerError {
    Expired,
    Failed(String),
}

/// What a server page does next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Next {
    /// Ask the student whether the session may go to the server.
    Ask,
    Fetch,
    /// Sign in on the server with the saved password.
    Relogin,
    /// Skip the server route for this reason; the phone fetches.
    Fail(String),
    /// The phone fetches, with nothing skipped.
    GiveUp,
}

/// A server page: asked once, fetched, and on an expired session signed in again once and
/// fetched again.
#[derive(Debug, Default)]
pub(super) struct ServerFetch {
    relogged: bool,
}

impl ServerFetch {
    /// The first step; a pending OTP sends nothing and asks nothing.
    pub(super) fn start(otp_pending: bool) -> Next {
        if otp_pending { Next::GiveUp } else { Next::Ask }
    }

    /// A no goes to the phone and skips nothing, so the next fetch asks again.
    pub(super) fn consented(&self, yes: bool) -> Next {
        if yes { Next::Fetch } else { Next::GiveUp }
    }

    pub(super) fn fetched(&mut self, error: ServerError) -> Next {
        match error {
            ServerError::Expired if !self.relogged => {
                self.relogged = true;
                Next::Relogin
            }
            ServerError::Expired => Next::GiveUp,
            ServerError::Failed(reason) => Next::Fail(reason),
        }
    }

    pub(super) fn relogged(&self, result: Result<(), ServerError>) -> Next {
        match result {
            Ok(()) => Next::Fetch,
            Err(ServerError::Expired) => Next::GiveUp,
            Err(ServerError::Failed(reason)) => Next::Fail(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000;

    /// (chosen, results so far as (route, outcome, at), asked about, at, active)
    type ActiveCase<'a> = (Route, &'a [(Route, Outcome, i64)], Route, i64, bool);
    /// (name, otp pending, consented, the edge's answers, the steps)
    type StepCase = (
        &'static str,
        bool,
        bool,
        Vec<Result<(), ServerError>>,
        Vec<Next>,
    );

    fn plan(chosen: Route) -> RoutePlan {
        let mut plan = RoutePlan::new();
        plan.choose(chosen);
        plan
    }

    fn failed(reason: &str) -> Outcome {
        Outcome::Failed(reason.into())
    }

    #[test]
    fn active_routes() {
        use Route::*;
        let cases: &[ActiveCase<'_>] = &[
            (Direct, &[], Direct, NOW, true),
            (Direct, &[], Server, NOW, false),
            (Proxy, &[], Proxy, NOW, true),
            (Proxy, &[], Server, NOW, false),
            (Proxy, &[(Proxy, failed("x"), NOW)], Proxy, NOW, false),
            (Server, &[], Server, NOW, true),
            (Server, &[], Udp, NOW, false),
            (Udp, &[], Udp, NOW, true),
            (Udp, &[], Server, NOW, true),
            (Udp, &[], Direct, NOW, false),
            (Server, &[(Server, failed("x"), NOW)], Server, NOW, false),
            (
                Server,
                &[(Server, failed("x"), NOW)],
                Server,
                NOW + SKIP_S - 1,
                false,
            ),
            (
                Server,
                &[(Server, failed("x"), NOW)],
                Server,
                NOW + SKIP_S,
                true,
            ),
            (
                Server,
                &[(Server, failed("x"), NOW), (Server, Outcome::Used, NOW)],
                Server,
                NOW,
                true,
            ),
            (Udp, &[(Udp, failed("x"), NOW)], Udp, NOW, false),
            (Udp, &[(Udp, failed("x"), NOW)], Server, NOW, true),
        ];
        for (chosen, results, route, at, want) in cases {
            let mut plan = plan(*chosen);
            for (route, outcome, at) in *results {
                plan.on_result(*route, outcome.clone(), *at);
            }
            assert_eq!(
                plan.active(*route, *at),
                *want,
                "{chosen:?} {results:?} {route:?} at {at}"
            );
        }
    }

    #[test]
    fn udp_falls_back_to_server_then_direct() {
        // (chosen, failed routes at NOW, at, next)
        let cases: &[(Route, &[Route], i64, Option<Route>)] = &[
            (Route::Direct, &[], NOW, None),
            (Route::Proxy, &[], NOW, None),
            (Route::Server, &[], NOW, Some(Route::Server)),
            (Route::Udp, &[], NOW, Some(Route::Udp)),
            (Route::Udp, &[Route::Udp], NOW, Some(Route::Server)),
            (Route::Udp, &[Route::Udp, Route::Server], NOW, None),
            (Route::Udp, &[Route::Udp], NOW + SKIP_S, Some(Route::Udp)),
            (Route::Server, &[Route::Server], NOW, None),
        ];
        for (chosen, fails, at, want) in cases {
            let mut plan = plan(*chosen);
            for route in *fails {
                plan.on_result(*route, failed("down"), NOW);
            }
            assert_eq!(plan.next(*at), *want, "{chosen:?} {fails:?} at {at}");
        }
    }

    #[test]
    fn status_lists_live_skips_and_the_last_use() {
        let mut plan = plan(Route::Udp);
        plan.on_result(Route::Server, failed("late"), NOW + 10);
        plan.on_result(Route::Udp, failed("lossy"), NOW);
        plan.on_result(Route::Direct, Outcome::Used, NOW + 20);
        let status = plan.status(NOW + 30);
        assert_eq!(status.chosen, Route::Udp);
        assert_eq!(
            (status.last_used, status.last_used_at),
            (Some(Route::Direct), Some(NOW + 20))
        );
        let skips: Vec<_> = status
            .skipped
            .iter()
            .map(|skip| (skip.route, skip.until, skip.reason.as_str()))
            .collect();
        assert_eq!(
            skips,
            [
                (Route::Udp, NOW + SKIP_S, "lossy"),
                (Route::Server, NOW + 10 + SKIP_S, "late"),
            ]
        );
        assert!(plan.status(NOW + 10 + SKIP_S).skipped.is_empty());
    }

    #[test]
    fn choose_and_forget_clear_skips() {
        let mut plan = plan(Route::Server);
        plan.on_result(Route::Server, failed("x"), NOW);
        plan.on_result(Route::Server, Outcome::Used, NOW);
        plan.on_result(Route::Server, failed("x"), NOW);
        plan.choose(Route::Server);
        assert!(plan.active(Route::Server, NOW));
        assert_eq!(plan.status(NOW).last_used, Some(Route::Server));
        plan.forget();
        assert_eq!(plan, RoutePlan::new());
    }

    /// The steps a server page takes, given the student's answer and the edge's answers in turn
    /// (`Ok` a page; the relogin's answers come after the fetch that expired).
    fn steps(otp: bool, yes: bool, answers: &[Result<(), ServerError>]) -> Vec<Next> {
        let mut fetch = ServerFetch::default();
        let mut answers = answers.iter().cloned();
        let mut next = ServerFetch::start(otp);
        let mut steps = vec![next.clone()];
        loop {
            next = match next {
                Next::Ask => fetch.consented(yes),
                Next::Fetch => match answers.next().unwrap() {
                    Ok(()) => return steps,
                    Err(error) => fetch.fetched(error),
                },
                Next::Relogin => fetch.relogged(answers.next().unwrap()),
                Next::Fail(_) | Next::GiveUp => return steps,
            };
            steps.push(next.clone());
        }
    }

    #[test]
    fn server_page_steps() {
        use Next::*;
        use ServerError::{Expired, Failed};
        let down = || Failed("server unreachable".into());
        let fail = || Fail("server unreachable".into());
        let cases: Vec<StepCase> = vec![
            ("otp pending", true, true, vec![], vec![GiveUp]),
            (
                "denied: Direct, no skip",
                false,
                false,
                vec![],
                vec![Ask, GiveUp],
            ),
            ("page", false, true, vec![Ok(())], vec![Ask, Fetch]),
            (
                "server failed",
                false,
                true,
                vec![Err(down())],
                vec![Ask, Fetch, fail()],
            ),
            (
                "expired once, then retried",
                false,
                true,
                vec![Err(Expired), Ok(()), Ok(())],
                vec![Ask, Fetch, Relogin, Fetch],
            ),
            (
                "expired twice",
                false,
                true,
                vec![Err(Expired), Ok(()), Err(Expired)],
                vec![Ask, Fetch, Relogin, Fetch, GiveUp],
            ),
            (
                "relogin failed",
                false,
                true,
                vec![Err(Expired), Err(down())],
                vec![Ask, Fetch, Relogin, fail()],
            ),
            (
                "relogin needs the phone",
                false,
                true,
                vec![Err(Expired), Err(Expired)],
                vec![Ask, Fetch, Relogin, GiveUp],
            ),
            (
                "retry failed",
                false,
                true,
                vec![Err(Expired), Ok(()), Err(down())],
                vec![Ask, Fetch, Relogin, Fetch, fail()],
            ),
        ];
        for (name, otp, yes, answers, want) in cases {
            assert_eq!(steps(otp, yes, &answers), want, "{name}");
        }
    }
}
