//! Every call to our server (`backend/`, Convex) goes through here, on one WebSocket
//! (`docs/plans/2026-11-23-backend-v3-api.md`). Split by responsibility:
//! - [`rpc`]: `public`, `signed`, `jwt`, `account` and `device` function calls and error mapping;
//! - [`identity`]: JWT and refresh lifecycle, kept sign-outs and background proofs, driven by the
//!   pure state machine in [`auth`];
//! - [`live`]: [`Core::live_watch`] subscriptions and their single JWT renewal coordinator.
//!
//! The exported calls (identity checks, the background proof, live watches) stay here: UniFFI
//! orders a type's methods by module, so moving one would reorder the Kotlin bindings.
//!
//! Every call carries the flow id (`flow`). Arguments, answers, tokens and JWTs are never
//! logged.

mod auth;
pub(crate) mod identity;
mod live;
mod rpc;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use futures::channel::oneshot;
use futures::future::BoxFuture;
use serde_json::{Value, json};
use tracing::{info, warn};

use crate::convex::{CallError, ConvexClient, SubscriptionId};
use crate::remote::server_code::ServerCode;
use crate::{Core, CoreError, Shared};

use auth::{AuthMachine, Effect};
use live::{LiveError, LiveListener, LiveWatch, Renewal, run_renewal_coordinator};
// Also here for `keys.rs`'s doc link, which the Kotlin bindings copy word for word.
pub(super) use live::LiveTopic;
use rpc::edge_error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Query,
    Mutation,
    Action,
}

/// The server's functions: the Convex client, or a test's stand-in.
pub(crate) trait Backend: Send + Sync {
    fn run<'a>(
        &'a self,
        kind: Kind,
        path: &'a str,
        args: Value,
    ) -> BoxFuture<'a, Result<Value, CallError>>;
    /// The JWT every later call goes with (None: none).
    fn set_auth(&self, jwt: Option<String>);
    /// Follows query `path(args)`: `on_update` hears its result now and after every change.
    fn watch(&self, path: &str, args: Value, on_update: OnUpdate) -> u64;
    fn unwatch(&self, id: u64);
    /// Closes the underlying transport and stops background timers (default no-op for test doubles).
    fn close(&self) {}
}

pub(crate) type OnUpdate = Box<dyn Fn(Result<Value, CallError>) + Send + Sync>;

impl Backend for ConvexClient {
    fn run<'a>(
        &'a self,
        kind: Kind,
        path: &'a str,
        args: Value,
    ) -> BoxFuture<'a, Result<Value, CallError>> {
        Box::pin(async move {
            match kind {
                Kind::Query => self.query(path, args).await,
                Kind::Mutation => self.mutation(path, args).await,
                Kind::Action => self.action(path, args).await,
            }
        })
    }

    fn set_auth(&self, jwt: Option<String>) {
        ConvexClient::set_auth(self, jwt);
    }

    fn watch(&self, path: &str, args: Value, on_update: OnUpdate) -> u64 {
        self.subscribe(path, args, on_update).0
    }

    fn unwatch(&self, id: u64) {
        self.unsubscribe(SubscriptionId(id));
    }

    fn close(&self) {
        ConvexClient::close(self);
    }
}

/// The core's server state ([`Shared::backend`]).
#[derive(Default)]
pub(crate) struct State {
    backend: Option<Arc<dyn Backend>>,
    auth: AuthMachine,
    renewal: Renewal,
    /// The calls in [`Core::wait_server_identity`], woken when the identity may have come.
    identity_waiters: Vec<oneshot::Sender<()>>,
}

impl Shared {
    fn backend_state(&self) -> std::sync::MutexGuard<'_, State> {
        self.backend.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Wakes the next time [`Shared::identity_may_have_changed`] runs.
    fn identity_change(&self) -> oneshot::Receiver<()> {
        let (wake, woken) = oneshot::channel();
        self.backend_state().identity_waiters.push(wake);
        woken
    }

    /// The identity, the registration number's answer or the signed-in student changed: the
    /// calls waiting for an identity look again.
    pub(crate) fn identity_may_have_changed(&self) {
        let waiters = std::mem::take(&mut self.backend_state().identity_waiters);
        for wake in waiters {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "a waiter that went away needs no wake-up"
            )]
            let _ = wake.send(());
        }
    }

    /// The Convex socket itself got `AuthError` and already cleared its own auth token: drop the
    /// ready JWT in the state machine so the next call refreshes.
    fn on_socket_auth_error(&self) {
        let (_, effects) = self.backend_state().auth.on_jwt_rejected(None);
        if effects.contains(&Effect::ResetUdpJwt) {
            crate::remote::udp::forget_jwt_sent(self);
        }
    }
}

/// Every call takes the flow id first.
#[uniffi::export]
impl Core {
    /// Whether this install holds a server identity it may use unasked for the signed-in
    /// student (a JWT, a refresh token or a v2 account token) and they shared their
    /// registration number, so a live watch can follow; read on the phone alone.
    pub async fn has_server_identity(&self, flow: String) -> bool {
        crate::traced("has_server_identity", &flow, async {
            let regno = self.signed_in_regno().await;
            if self
                .regno_allowed(&flow, regno.as_deref(), false)
                .await
                .is_err()
            {
                return Ok(false);
            }
            let server = self.forwarding_server();
            let refresh = self.device().refresh;
            let has_v2 = self.kept_v2(regno.as_deref()).is_some();
            Ok(self.shared.backend_state().auth.has_usable_identity(
                &server,
                regno.as_deref(),
                refresh.as_ref(),
                has_v2,
            ))
        })
        .await
        .unwrap_or(false)
    }

    /// Returns once [`Core::has_server_identity`] holds: at once, or when a proof, the student's
    /// Share or a sign-in brings an identity. The app's watch of session requests waits here.
    pub async fn wait_server_identity(&self, flow: String) {
        loop {
            // Registered before the check, so a change between the two still wakes it.
            let changed = self.shared.identity_change();
            if self.has_server_identity(flow.clone()).await {
                return;
            }
            // Dropped unsent only with the core: looking again is all there is to do.
            if changed.await.is_err() {
                info!(flow, "identity wait woken without a change");
            }
        }
    }

    /// Follows `topic` as the student this install proved to be: `listener` hears each change
    /// as soon as the server has it. Without an identity (it never proves) or a forwarding
    /// address for [`LiveTopic::MailInbox`], an error, and the caller reads once instead.
    pub async fn live_watch(
        self: Arc<Self>,
        flow: String,
        topic: LiveTopic,
        listener: Arc<dyn LiveListener>,
    ) -> Result<Arc<LiveWatch>, CoreError> {
        crate::traced("live_watch", &flow, async {
            let (path, args) = match topic {
                LiveTopic::Account => ("account:view", json!({})),
                LiveTopic::MailInbox => {
                    let (_, id, _) =
                        self.saved_mailbox()
                            .ok_or_else(|| CoreError::ForwardingOff {
                                kind: crate::ForwardingOffKind::NotSetUp,
                                reason: "Mail forwarding is off.".into(),
                            })?;
                    ("mail:inbox", json!({ "id": id }))
                }
                LiveTopic::SessionRequests => ("sessions:pending", json!({})),
                LiveTopic::Server => (
                    "screen:view",
                    crate::remote::server::screen_args(
                        self.saved_mailbox().as_ref().map(|(_, id, _)| id.as_str()),
                    ),
                ),
            };
            let stamp = self.ensure_identity(&flow, path, false).await?;
            let backend = self.backend()?;
            let held = Arc::new(());
            let (wake, start) = {
                let mut state = self.shared.backend_state();
                let start = state.renewal.watch(&held, &flow);
                (state.renewal.wake.clone(), start)
            };
            if start {
                run_renewal_coordinator(Arc::downgrade(&self), self.shared.clone());
            }
            let (core, renew_flow) = (Arc::downgrade(&self), flow.clone());
            // Tracks the JWT epoch this watch last succeeded under, so a rejected JWT invalidates
            // that epoch once and concurrent watches share one refresh.
            let seen_jwt_epoch = Arc::new(AtomicU64::new(stamp.jwt_epoch));
            let watch_gen = stamp.generation;
            let on_update = move |update: Result<Value, CallError>| match update {
                Ok(_) => {
                    if let Some(core) = core.upgrade() {
                        let current = core.shared.backend_state().auth.stamp().jwt_epoch;
                        seen_jwt_epoch.store(current, Ordering::Relaxed);
                    }
                    listener.on_change();
                }
                Err(error) => {
                    let error = edge_error(path, error);
                    if let Some(core) = core.upgrade() {
                        if error.is(&ServerCode::Unauthenticated) {
                            let failed_epoch = seen_jwt_epoch.load(Ordering::Relaxed);
                            core.on_jwt_rejected(Some(failed_epoch));
                            core.request_watch_renewal(&renew_flow, listener.clone());
                        }
                        // Removed on another phone: only act if this watch belongs to the current
                        // identity generation, so a late error never wipes a newer refresh token.
                        if error.is(&ServerCode::DeviceRemoved) {
                            if !core.on_device_removed(watch_gen) {
                                return;
                            }
                            warn!(path, gen = watch_gen, "this install was removed on the server");
                        }
                    }
                    listener.on_error(LiveError::of(error));
                }
            };
            let id = backend.watch(path, args, Box::new(on_update));
            info!(path, "live watch");
            Ok(Arc::new(LiveWatch {
                backend,
                id,
                held: Mutex::new(Some(held)),
                wake,
            }))
        })
        .await
    }
}

#[uniffi::export]
impl Core {
    /// Proves who the student is in the background, so the refresh token is there before
    /// Connected apps, forwarding or a server route needs it: right after a sign-in, and again
    /// at each app open until it worked. Only when this install has no identity to use (no
    /// refresh or v2 token, no live JWT) and is signed in; it gives up after `timeout_ms` and
    /// the next open tries again. It takes the identity's turn, so a call someone starts
    /// meanwhile waits for it and uses its JWT instead of proving again. No session leaves the
    /// phone in a proof, so no cookie consent is asked; it proves only once the student said
    /// Share to their registration number ([`crate::remote::regno`]), and never asks. Kotlin runs it in an unattended flow, on an
    /// unmetered or validated non-cellular network unless Developer allows mobile data. True
    /// when it proved.
    pub async fn prove_in_background(
        &self,
        flow: String,
        timeout_ms: u32,
    ) -> Result<bool, CoreError> {
        crate::traced("prove_in_background", &flow, async {
            let budget = Duration::from_millis(timeout_ms.into());
            match self
                .identity_within(&flow, "Background proof", Some(budget), true)
                .await
            {
                // Not until the student said Share; regno_allowed logged it.
                Err(CoreError::RegnoNotShared) => Ok(false),
                proved => proved.map(|(_, taken)| taken),
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests;
