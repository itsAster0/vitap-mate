//! Macros the other modules use; declared first in `lib.rs`.

/// Runs `$call` with `$client` bound to the signed-in client. When the
/// session has expired, signs in again with the saved credentials and runs it
/// once more. Nothing is returned for a client signed out meanwhile.
macro_rules! signed_in {
    ($core:expr, |$client:ident| $call:expr) => {{
        let core = $core;
        let slot = core.current()?;
        $crate::demo::refuse(&slot.read().await.account)?;
        let first = {
            let session = slot.read().await;
            let $client = &session.client;
            if session.is_awaiting_otp() {
                // Waiting for the OTP: a call now would check the session with VTOP, which
                // bounces to the login page and wipes the challenge the code is for.
                // `relogin` answers with the open challenge instead.
                Err(session.logins)
            } else {
                match $call.await {
                    Err(VtopError::SessionExpired) => Err(session.logins),
                    result => Ok(result?),
                }
            }
        };
        let data = match first {
            Ok(data) => data,
            Err(seen) => {
                core.relogin(&slot, seen).await?;
                let session = slot.read().await;
                let $client = &session.client;
                $call.await.map_err(|error| match error {
                    VtopError::SessionExpired => CoreError::Other {
                        reason: "VTOP ended the session right after signing in".to_string(),
                    },
                    error => error.into(),
                })?
            }
        };
        if core.is_current(&slot) {
            // A session that checked its cookies with VTOP learned the regno: kept with the
            // account if it had none.
            core.keep_regno(&slot).await;
            Ok(data)
        } else {
            Err(CoreError::NotSignedIn)
        }
    }};
}
