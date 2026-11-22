//! VTOP's cookies in the app's one cookie store: Android's `CookieManager`, which the web view
//! uses too, so both are one VTOP session (no copying). Kotlin implements [`SharedCookies`]
//! over it; vtop-core reads the `Cookie` header from it before each hop and stores each hop's
//! `Set-Cookie` in it, so a redirect's cookies land before the next hop.
//!
//! The store is process-wide, sessions are not: a sign-in runs on a jar of its own until it is
//! installed (a rejected password leaves the signed-in session alone), and a replaced session
//! sees no cookies at all, so it can neither use nor clear the new one's. Without a shared
//! store set (host tests), every session keeps its own jar. Values are never logged.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use tracing::info;
use vtop_core::session::{CookieStore, ResettableJar};

/// The app's cookie store (`CookieManager`). Called off the main thread.
#[uniffi::export(with_foreign)]
pub trait SharedCookies: Send + Sync {
    /// The `Cookie` header for `url`; None without cookies.
    fn cookie(&self, url: String) -> Option<String>;
    /// Stores each `Set-Cookie` value as if `url` had sent it.
    fn set_cookies(&self, url: String, set_cookies: Vec<String>);
    /// Removes VTOP's cookies and saves.
    fn clear(&self);
    /// Saves the cookies to disk now.
    fn flush(&self);
}

struct Global {
    store: Option<Arc<dyn SharedCookies>>,
    /// The session whose cookies are the store's.
    active: Weak<SessionCookies>,
}

const NONE: Global = Global {
    store: None,
    active: Weak::new(),
};

#[cfg(not(test))]
fn global<R>(work: impl FnOnce(&mut Global) -> R) -> R {
    static GLOBAL: Mutex<Global> = Mutex::new(NONE);
    work(&mut lock(&GLOBAL))
}

/// Per test thread, so tests running at once don't share a store.
#[cfg(test)]
fn global<R>(work: impl FnOnce(&mut Global) -> R) -> R {
    thread_local!(static GLOBAL: Mutex<Global> = const { Mutex::new(NONE) });
    GLOBAL.with(|global| work(&mut lock(global)))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Keeps VTOP cookies in `cookies` (the web view's store); call before [`crate::Core::new`].
#[uniffi::export]
pub fn set_shared_cookies(cookies: Arc<dyn SharedCookies>) {
    info!("cookies: shared");
    global(|global| global.store = Some(cookies));
}

/// Where one session's cookies are.
enum Place {
    /// Not the app's session (yet): a jar of its own.
    Own(ResettableJar),
    /// The signed-in session: the shared store.
    Shared(Arc<dyn SharedCookies>),
    /// Replaced by another session: none, and nothing is stored.
    Gone,
}

/// One session's cookies; see the module docs.
pub(crate) struct SessionCookies(Mutex<Place>);

impl std::fmt::Debug for SessionCookies {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let place = match &*lock(&self.0) {
            Place::Own(_) => "own",
            Place::Shared(_) => "shared",
            Place::Gone => "gone",
        };
        write!(formatter, "SessionCookies({place})")
    }
}

impl SessionCookies {
    /// A session's cookies before it is installed: a jar of its own.
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(Place::Own(ResettableJar::default()))))
    }

    /// Makes this the session in the shared store; the one before sees no cookies from now on.
    /// With `carry`, the store's VTOP cookies are replaced by what this session has collected
    /// (a sign-in, or a `session.json` from before the shared store, migrated once); else the
    /// store's stay (they are this session's, restored at launch). No shared store: nothing.
    pub(crate) fn activate(self: &Arc<Self>, carry: bool) {
        let store = global(|global| {
            let store = global.store.clone()?;
            if let Some(before) = global.active.upgrade()
                && !Arc::ptr_eq(&before, self)
            {
                *lock(&before.0) = Place::Gone;
            }
            global.active = Arc::downgrade(self);
            Some(store)
        });
        let Some(store) = store else { return };
        let mut place = lock(&self.0);
        if let Place::Own(jar) = &*place
            && carry
        {
            let lines = jar.set_cookie_lines();
            store.clear();
            for (url, line) in &lines {
                store.set_cookies(url.clone(), vec![line.clone()]);
            }
            store.flush();
            info!(cookies = lines.len(), "cookies moved to the shared store");
        }
        *place = Place::Shared(store);
    }

    /// Whether these are the shared store's cookies.
    pub(crate) fn is_shared(&self) -> bool {
        matches!(&*lock(&self.0), Place::Shared(_))
    }

    /// Saves the shared store to disk (after a sign-in).
    pub(crate) fn flush(&self) {
        if let Some(store) = self.store() {
            store.flush();
        }
    }

    fn store(&self) -> Option<Arc<dyn SharedCookies>> {
        match &*lock(&self.0) {
            Place::Shared(store) => Some(store.clone()),
            _ => None,
        }
    }
}

/// Signed out: no session has the shared store, and its VTOP cookies go.
pub(crate) fn sign_out() {
    let store = global(|global| {
        if let Some(before) = global.active.upgrade() {
            *lock(&before.0) = Place::Gone;
        }
        global.active = Weak::new();
        global.store.clone()
    });
    if let Some(store) = store {
        store.clear();
    }
}

impl CookieStore for SessionCookies {
    fn header_for(&self, url: &str) -> Option<String> {
        // The foreign calls run without the lock held.
        if let Some(store) = self.store() {
            return store
                .cookie(url.to_string())
                .filter(|header| !header.is_empty());
        }
        match &*lock(&self.0) {
            Place::Own(jar) => jar.header_for(url),
            _ => None,
        }
    }

    fn set_cookies(&self, url: &str, set_cookies: &[&str]) {
        if let Some(store) = self.store() {
            let lines = set_cookies.iter().map(|line| line.to_string()).collect();
            return store.set_cookies(url.to_string(), lines);
        }
        if let Place::Own(jar) = &*lock(&self.0) {
            CookieStore::set_cookies(jar, url, set_cookies);
        }
    }

    fn clear(&self) {
        if let Some(store) = self.store() {
            return store.clear();
        }
        if let Place::Own(jar) = &*lock(&self.0) {
            jar.clear();
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A shared store in memory that counts its clears.
    #[derive(Default)]
    pub(crate) struct Memory {
        pub(crate) jar: ResettableJar,
        pub(crate) clears: Mutex<u32>,
    }

    impl SharedCookies for Memory {
        fn cookie(&self, url: String) -> Option<String> {
            self.jar.header_for(&url)
        }
        fn set_cookies(&self, url: String, set_cookies: Vec<String>) {
            self.jar
                .set_cookies(set_cookies.iter().map(String::as_str), &url);
        }
        fn clear(&self) {
            *lock(&self.clears) += 1;
            self.jar.clear();
        }
        fn flush(&self) {}
    }

    /// Sets a fresh shared store for this test thread.
    pub(crate) fn with_shared() -> Arc<Memory> {
        let memory = Arc::new(Memory::default());
        global(|global| *global = NONE);
        set_shared_cookies(memory.clone());
        memory
    }

    const VTOP: &str = "https://vtop.example.in/vtop/content";

    #[test]
    fn only_the_active_session_uses_the_shared_store() {
        let memory = with_shared();
        memory
            .jar
            .add_cookie_str("JSESSIONID=web; Path=/vtop", VTOP);

        // A sign-in collects its cookies apart; the store keeps the signed-in session's.
        let signing_in = SessionCookies::new();
        signing_in.set_cookies(VTOP, &["JSESSIONID=new; Path=/vtop"]);
        assert_eq!(
            memory.cookie(VTOP.into()).as_deref(),
            Some("JSESSIONID=web")
        );

        // Restored at launch: the store's cookies stay and are this session's.
        let restored = SessionCookies::new();
        restored.activate(false);
        assert!(restored.is_shared());
        assert_eq!(restored.header_for(VTOP).as_deref(), Some("JSESSIONID=web"));
        restored.set_cookies(VTOP, &["SERVERID=s1; Path=/"]);
        assert_eq!(
            memory.cookie(VTOP.into()).as_deref(),
            Some("JSESSIONID=web; SERVERID=s1")
        );

        // The sign-in is installed: its cookies replace the store's, the one before sees none
        // and can no longer clear or overwrite them.
        signing_in.activate(true);
        assert_eq!(
            memory.cookie(VTOP.into()).as_deref(),
            Some("JSESSIONID=new")
        );
        assert!(!restored.is_shared());
        assert_eq!(restored.header_for(VTOP), None);
        restored.set_cookies(VTOP, &["JSESSIONID=old; Path=/vtop"]);
        restored.clear();
        assert_eq!(
            memory.cookie(VTOP.into()).as_deref(),
            Some("JSESSIONID=new")
        );

        sign_out();
        assert_eq!(memory.cookie(VTOP.into()), None);
        assert!(!signing_in.is_shared());
    }

    #[test]
    fn without_a_shared_store_a_session_keeps_its_own_jar() {
        let cookies = SessionCookies::new();
        cookies.activate(true);
        cookies.set_cookies(VTOP, &["JSESSIONID=a; Path=/vtop"]);
        assert!(!cookies.is_shared());
        assert_eq!(cookies.header_for(VTOP).as_deref(), Some("JSESSIONID=a"));
    }
}
