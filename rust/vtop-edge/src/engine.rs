//! `Engine`: runs the engine (`vtop-bridge`) in a Cloudflare Container and
//! passes requests to it. One instance, named "engine", serves everyone.
//!
//! The container starts on the first request, with this Worker's secrets as
//! its environment, and stops [`IDLE_SECS`] after the last one (the
//! container's inactivity timeout; without it the container stops soon after
//! the object goes idle). Calls reach it here when `BRIDGE_URL` / `MCP_URL`
//! are set to [`IN_CONTAINER`].

use std::cell::Cell;
use std::time::Duration;

use js_sys::{Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use worker::{
    durable_object, Container, DurableObject, Env, Headers, Request, RequestInit, Response, Result,
    State,
};

use crate::var;

/// The value of `BRIDGE_URL` / `MCP_URL` that sends calls to the container.
pub(crate) const IN_CONTAINER: &str = "container";

const PORT: u16 = 8080;
/// Long enough for a sign-in the engine finishes after answering.
const IDLE_SECS: u64 = 600;
/// How long a request waits for a starting container to listen.
const START_WAIT: Duration = Duration::from_secs(20);
const START_RETRY: Duration = Duration::from_millis(250);

#[durable_object]
pub struct Engine {
    state: State,
    env: Env,
    /// This object instance has set the container's inactivity timeout;
    /// a restarted one (after a deploy) has to set it again.
    timeout_set: Cell<bool>,
}

impl DurableObject for Engine {
    fn new(state: State, env: Env) -> Self {
        Self {
            state,
            env,
            timeout_set: Cell::new(false),
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let container = self
            .state
            .container()
            .ok_or("this object has no container")?;
        if !container.running() {
            self.start(&container)?;
            self.timeout_set.set(false);
            worker::console_log!("engine: container started");
        }
        if !self.timeout_set.get() {
            set_inactivity_timeout(&container, IDLE_SECS * 1000).await?;
            self.timeout_set.set(true);
        }

        // Kept to send again while the container is still starting.
        let url = req.url()?;
        let path = match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_string(),
        };
        let method = req.method();
        let headers = req.headers().clone();
        let body = req.bytes().await.unwrap_or_default();
        let port = container.get_tcp_port(PORT)?;
        let started = js_sys::Date::now();
        loop {
            let mut init = RequestInit::new();
            init.with_method(method.clone())
                .with_headers(copy(&headers)?);
            if !body.is_empty() {
                init.with_body(Some(js_sys::Uint8Array::from(body.as_slice()).into()));
            }
            let attempt = Request::new_with_init(&format!("http://engine{path}"), &init)?;
            match port.fetch_request(attempt).await {
                Ok(response) => return Ok(response),
                Err(_) if js_sys::Date::now() - started < START_WAIT.as_millis() as f64 => {
                    worker::Delay::from(START_RETRY).await;
                }
                Err(e) => {
                    worker::console_log!("engine: container did not answer: {e}");
                    return crate::error(
                        503,
                        "engine_unavailable",
                        "The engine is starting; try again.",
                    );
                }
            }
        }
    }

    /// Nothing is scheduled; this only clears an alarm left by an older
    /// version.
    async fn alarm(&self) -> Result<Response> {
        Response::empty()
    }
}

impl Engine {
    /// Starts the container with this Worker's secrets as its environment.
    /// Calls the runtime directly: `ContainerStartupOptions` in worker
    /// 0.8.7 drops the fields it is given (and sends `env` as a Map).
    fn start(&self, container: &Container) -> Result<()> {
        let env = Object::new();
        let vault_key = self.env.secret("VAULT_KEY")?.to_string();
        let relay_secret = self.env.secret("BRIDGE_SECRET")?.to_string();
        let edge_url = var(&self.env, "PUBLIC_URL")?;
        let port = PORT.to_string();
        for (key, value) in [
            ("PORT", port.as_str()),
            // Warnings, plus the engine's one line per request; info from
            // rmcp and VTOP fetches is noise (and billed as logs).
            ("RUST_LOG", "warn,vtop_bridge=info"),
            ("VAULT_KEY", vault_key.as_str()),
            ("MAIL_RELAY_SECRET", relay_secret.as_str()),
            ("EDGE_URL", edge_url.as_str()),
        ] {
            Reflect::set(&env, &key.into(), &value.into())?;
        }
        let options = Object::new();
        Reflect::set(&options, &"env".into(), &env)?;
        Reflect::set(&options, &"enableInternet".into(), &JsValue::TRUE)?;
        let target: &JsValue = container.as_ref();
        let start: Function = Reflect::get(target, &"start".into())?.dyn_into()?;
        start.call1(target, &options)?;
        Ok(())
    }
}

/// `setInactivityTimeout`, which worker 0.8.7 does not wrap: how long the
/// container keeps running once this object stops getting requests.
async fn set_inactivity_timeout(container: &Container, ms: u64) -> Result<()> {
    let target: &JsValue = container.as_ref();
    let set: Function = Reflect::get(target, &"setInactivityTimeout".into())?.dyn_into()?;
    let promise: js_sys::Promise = set
        .call1(target, &JsValue::from_f64(ms as f64))?
        .dyn_into()?;
    wasm_bindgen_futures::JsFuture::from(promise).await?;
    Ok(())
}

fn copy(headers: &Headers) -> Result<Headers> {
    let out = Headers::new();
    for (name, value) in headers.entries() {
        out.append(&name, &value)?;
    }
    Ok(out)
}
