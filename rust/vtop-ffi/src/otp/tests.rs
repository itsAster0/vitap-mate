use std::sync::{Arc, Mutex};

use vtop_core::sync::RwLock;

use super::{OtpChallenge, OtpListener};
use crate::remote::consent;
use crate::{Account, Core, CoreError, OtpVia, Session, Slot};

/// Every prompt the app was told, in order.
#[derive(Default)]
struct Told(Mutex<Vec<Option<OtpChallenge>>>);

impl OtpListener for Told {
    fn on_challenge(&self, challenge: Option<OtpChallenge>) {
        self.0.lock().unwrap().push(challenge);
    }
}

impl Told {
    fn all(&self) -> Vec<Option<OtpChallenge>> {
        self.0.lock().unwrap().clone()
    }

    fn last(&self) -> Option<OtpChallenge> {
        self.all().last().cloned().flatten()
    }
}

const ASKED: &str = "Additional verification is required.";

/// A signed-in core whose session waits for the OTP, with a gate saying whether flows are
/// watched and a listener for the prompt.
fn awaiting(name: &str, attended: bool) -> (Core, Arc<Slot>, Arc<Told>) {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-otp-{name}-{}", std::process::id()));
    let core = Core::for_test(dir);
    let account = Account {
        username: "21BCE0001".into(),
        password: "pw".into(),
        registration_number: None,
    };
    let slot = Arc::new(RwLock::new(Session::new(account, &core.shared).unwrap()));
    core.install(Some(slot.clone()));
    consent::tests::install(&core, attended, false);
    let told = Arc::new(Told::default());
    core.shared.otp.set_listener(told.clone());
    futures::executor::block_on(slot.write()).begin_otp(OtpVia::Phone, ASKED.into());
    (core, slot, told)
}

fn otp_required() -> CoreError {
    CoreError::OtpRequired {
        reason: ASKED.into(),
        issued_at: None,
    }
}

/// Runs `wait` until it parks, then `then`, then `wait` to its end.
async fn while_waiting<T>(wait: impl Future<Output = T>, then: impl FnOnce()) -> T {
    let mut wait = std::pin::pin!(wait);
    assert!(
        futures::poll!(wait.as_mut()).is_pending(),
        "the call waits for the code"
    );
    then();
    wait.await
}

#[tokio::test]
async fn nobody_watching_fails_at_once_and_opens_no_prompt() {
    let (core, slot, told) = awaiting("unattended", false);
    let failed = core.wait_for_otp(&slot, "sync.f", 0, otp_required()).await;
    assert_eq!(failed, Err(otp_required()));
    assert_eq!(told.all(), vec![None]);
}

#[tokio::test]
async fn a_watched_relogin_waits_for_the_code_and_goes_on_once_it_is_in() {
    let (core, slot, told) = awaiting("accepted", true);
    // A fetch's re-login (ensure_fresh_session signs in again the same way) meets the open
    // challenge: it opens the prompt and waits instead of failing.
    let fetch = core.ensure_fresh_session("attendance.refresh".into(), 0);
    let outcome = while_waiting(fetch, || {
        let prompt = told.last().expect("the prompt opened");
        assert_eq!((prompt.id, prompt.message.as_str()), (1, ASKED));
        // The code went in (the sheet, auto-fill or the server route all end here).
        futures::executor::block_on(slot.write()).mark_signed_in();
    })
    .await;
    assert_eq!(outcome, Ok(()));
    assert_eq!(told.all().last(), Some(&None), "the prompt closed");
}

#[tokio::test]
async fn cancelling_the_prompt_fails_the_waiting_calls() {
    let (core, slot, told) = awaiting("cancelled", true);
    let wait = core.wait_for_otp(&slot, "marks.refresh", 0, otp_required());
    let outcome = while_waiting(wait, || {
        futures::executor::block_on(core.cancel_otp("fetch_otp.cancel".into())).unwrap();
    })
    .await;
    assert_eq!(outcome, Err(otp_required()));
    assert_eq!(told.last(), None);
    // Closed: the next one prompts afresh after the next re-login, and nothing is pending now.
    assert_eq!(
        core.await_otp("account.save".into()).await,
        Err(CoreError::OtpRequired {
            reason: "The OTP prompt was closed.".into(),
            issued_at: None,
        })
    );
}

#[tokio::test]
async fn a_fresh_sign_in_under_the_prompt_keeps_it_open() {
    let (core, slot, told) = awaiting("fresh", true);
    let wait = core.wait_for_otp(&slot, "marks.refresh", 0, otp_required());
    let outcome = while_waiting(wait, || {
        let first = told.last().unwrap().id;
        // The code's sign-in had ended: a new one mails a new code under the same prompt.
        let mut session = futures::executor::block_on(slot.write());
        session.restart(&core.shared).unwrap();
        session.begin_otp(OtpVia::Phone, "VTOP sent a new code.".into());
        drop(session);
        let now = told.last().unwrap();
        assert_eq!(
            (now.id, now.message.as_str()),
            (first, "VTOP sent a new code.")
        );
        futures::executor::block_on(slot.write()).mark_signed_in();
    })
    .await;
    assert_eq!(outcome, Ok(()));
    assert!(
        !told.all().iter().flatten().any(|c| c.id != 1),
        "one prompt throughout: {:?}",
        told.all()
    );
}

#[tokio::test]
async fn sign_out_or_a_rejected_password_ends_the_wait() {
    let (core, slot, told) = awaiting("signed-out", true);
    let wait = core.wait_for_otp(&slot, "marks.refresh", 0, otp_required());
    let outcome = while_waiting(wait, || core.install(None)).await;
    assert_eq!(outcome, Err(CoreError::NotSignedIn));
    assert_eq!(told.last(), None);

    let (core, slot, _told) = awaiting("rejected", true);
    let wait = core.wait_for_otp(&slot, "marks.refresh", 0, otp_required());
    let outcome = while_waiting(wait, || {
        futures::executor::block_on(slot.write()).mark_password_rejected();
    })
    .await;
    assert_eq!(outcome, Err(CoreError::InvalidCredentials));
}

#[tokio::test]
async fn a_rejected_password_never_signs_in_afresh_for_a_code() {
    let dir = std::env::temp_dir().join(format!(
        "vtop-ffi-otp-rejected-fresh-{}",
        std::process::id()
    ));
    let core = Core::for_test(dir);
    let account = Account {
        username: "21BCE0001".into(),
        password: "pw".into(),
        registration_number: None,
    };
    let session = Session::new(account, &core.shared).unwrap().rejected(true);
    core.install(Some(Arc::new(RwLock::new(session))));
    // No challenge left to answer: a fresh sign-in would try the rejected password on VTOP.
    assert_eq!(
        core.submit_otp("f".into(), "123456".into()).await,
        Err(CoreError::InvalidCredentials)
    );
    assert_eq!(
        core.resend_otp("f".into()).await,
        Err(CoreError::InvalidCredentials)
    );
}

#[tokio::test]
async fn a_wait_past_its_deadline_gives_up_but_the_prompt_stays() {
    let (core, slot, told) = awaiting("deadline", true);
    let (over, deadline) = futures::channel::oneshot::channel::<()>();
    let wait = core.wait_for_otp_until(&slot, "marks.refresh", 0, otp_required(), async {
        // The test's clock: the sender dropped is the deadline passing.
        assert!(deadline.await.is_err());
    });
    let outcome = while_waiting(wait, || drop(over)).await;
    assert_eq!(outcome, Err(otp_required()));
    let prompt = told.last().expect("still open for the student");
    // Another call joins the same prompt, and the code going in later lets it through.
    let again = core.wait_for_otp(&slot, "exams.refresh", 0, otp_required());
    let outcome = while_waiting(again, || {
        assert_eq!(told.last().unwrap().id, prompt.id);
        futures::executor::block_on(slot.write()).mark_signed_in();
    })
    .await;
    assert_eq!(outcome, Ok(()));
}

#[tokio::test]
async fn await_otp_opens_the_prompt_for_a_sign_in_waiting_on_its_code() {
    // The Account page's sign-in said NeedsOtp: nothing is prompted until it waits.
    let (core, slot, told) = awaiting("account", true);
    assert_eq!(told.all(), vec![None]);
    let wait = core.await_otp("account.save".into());
    let outcome = while_waiting(wait, || {
        assert_eq!(told.last().map(|c| c.message), Some(ASKED.to_string()));
        futures::executor::block_on(slot.write()).mark_signed_in();
    })
    .await;
    assert_eq!(outcome, Ok(()));
    assert_eq!(told.last(), None);
}
