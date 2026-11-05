//! The app's one log pipeline, for Rust and Kotlin alike: JSON lines in
//! `<data_dir>/logs/app.jsonl` (fields `t l src msg flow regno`, plus span
//! fields and `ms` on span close; rolled to `.1` past [`MAX_BYTES`]), logcat
//! (tag `vitapmate`) and ATrace sections for Perfetto on Android.
//! vtop-core's `log` records are bridged in. Lines before [`obs_init`] reach
//! logcat only.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, Once, PoisonError};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

const MAX_BYTES: u64 = 1024 * 1024;

struct LogFile {
    path: PathBuf,
    file: File,
}

static LOG: Mutex<Option<LogFile>> = Mutex::new(None);
static REGNO: Mutex<Option<String>> = Mutex::new(None);

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // A poisoned lock only means another write panicked; keep logging.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Starts the pipeline, writing to `data_dir/logs/app.jsonl`. Safe to call
/// more than once.
#[uniffi::export]
pub fn obs_init(data_dir: String) {
    static SUBSCRIBER: Once = Once::new();
    SUBSCRIBER.call_once(|| {
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::filter::LevelFilter::INFO)
            .with(Pipeline)
            .try_init();
    });
    let mut log = locked(&LOG);
    if log.is_some() {
        return;
    }
    let path = PathBuf::from(data_dir).join("logs").join("app.jsonl");
    let opened = fs::create_dir_all(path.parent().unwrap_or(&path))
        .and_then(|()| OpenOptions::new().create(true).append(true).open(&path));
    match opened {
        Ok(file) => *log = Some(LogFile { path, file }),
        Err(error) => logcat("error", &format!("obs log file not opened: {error}")),
    }
}

/// Writes one line from Kotlin into the same file and logcat, in order.
#[uniffi::export]
pub fn log_event(
    level: String,
    src: String,
    flow: Option<String>,
    msg: String,
    fields: HashMap<String, String>,
) {
    let mut extra: Map<String, Value> = flow
        .map(|flow| ("flow".to_string(), flow.into()))
        .into_iter()
        .collect();
    extra.extend(fields.into_iter().map(|(key, value)| (key, value.into())));
    emit(&level, &src, &msg, extra);
}

/// The signed-in student's registration number, for every line.
pub(crate) fn set_regno(regno: Option<String>) {
    *locked(&REGNO) = regno;
}

fn emit(level: &str, src: &str, msg: &str, fields: Map<String, Value>) {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64);
    let mut line = Map::new();
    line.insert("t".into(), millis.into());
    line.insert("l".into(), level.into());
    line.insert("src".into(), src.into());
    line.insert("msg".into(), msg.into());
    if let Some(regno) = locked(&REGNO).clone() {
        line.insert("regno".into(), regno.into());
    }
    let text: String = fields
        .iter()
        .map(|(key, value)| format!(" {key}={}", plain(value)))
        .collect();
    logcat(level, &format!("{src} {msg}{text}"));
    line.extend(fields);
    let mut log = locked(&LOG);
    let Some(log) = log.as_mut() else { return };
    // Logging never breaks the app: a full disk only loses lines.
    if log.file.metadata().is_ok_and(|meta| meta.len() > MAX_BYTES) {
        let _ = fs::rename(&log.path, log.path.with_extension("jsonl.1"));
        if let Ok(file) = File::create(&log.path) {
            log.file = file;
        }
    }
    let _ = writeln!(log.file, "{}", Value::Object(line));
}

fn plain(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_string)
}

/// Fields and start time of an open span.
struct SpanData {
    fields: Map<String, Value>,
    started: Instant,
}

struct Fields<'a>(&'a mut Map<String, Value>);

impl Visit for Fields<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.insert(field, format!("{value:?}").into());
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert(field, value.into());
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field, value.into());
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field, value.into());
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field, value.into());
    }
}

impl Fields<'_> {
    fn insert(&mut self, field: &Field, value: Value) {
        // tracing-log's own fields (log.target, log.file, ...).
        if !field.name().starts_with("log.") {
            self.0.insert(field.name().to_string(), value);
        }
    }
}

fn level_name(level: &Level) -> String {
    level.as_str().to_lowercase()
}

/// Builds every line from `tracing` events and span closes.
struct Pipeline;

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for Pipeline {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut fields = Map::new();
        attrs.record(&mut Fields(&mut fields));
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SpanData {
                fields,
                started: Instant::now(),
            });
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        if let Some(data) = span.extensions_mut().get_mut::<SpanData>() {
            values.record(&mut Fields(&mut data.fields));
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let normalized = event.normalized_metadata();
        let meta = normalized.as_ref().unwrap_or_else(|| event.metadata());
        let mut line = Map::new();
        for span in ctx
            .event_scope(event)
            .into_iter()
            .flat_map(|scope| scope.from_root())
        {
            if let Some(data) = span.extensions().get::<SpanData>() {
                line.extend(data.fields.clone());
            }
        }
        // A span's outcome belongs to its close line only.
        line.remove("outcome");
        line.remove("error");
        let span_src = line.remove("src");
        let mut fields = Map::new();
        event.record(&mut Fields(&mut fields));
        let msg = fields
            .remove("message")
            .as_ref()
            .map(plain)
            .unwrap_or_default();
        // Our own events take the call's src; vtop-core's keep their target.
        let src = match (fields.remove("src"), span_src) {
            (Some(src), _) => plain(&src),
            (None, Some(src)) if meta.target().starts_with("vtop_ffi") => plain(&src),
            _ => meta.target().to_string(),
        };
        line.extend(fields);
        emit(&level_name(meta.level()), &src, &msg, line);
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else { return };
        let Some(SpanData {
            mut fields,
            started,
        }) = span.extensions_mut().remove::<SpanData>()
        else {
            return;
        };
        let src = fields
            .remove("src")
            .as_ref()
            .map_or_else(|| span.name().to_string(), plain);
        let failed = fields.get("outcome").is_some_and(|outcome| outcome != "ok");
        fields.insert("ms".into(), (started.elapsed().as_millis() as u64).into());
        emit(if failed { "warn" } else { "info" }, &src, "done", fields);
    }

    #[cfg(target_os = "android")]
    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let name = span
            .extensions()
            .get::<SpanData>()
            .and_then(|data| data.fields.get("src").map(plain));
        let name = std::ffi::CString::new(name.unwrap_or_else(|| span.name().to_string()))
            .unwrap_or_default();
        // SAFETY: `name` is a valid C string that outlives the call.
        unsafe { android::ATrace_beginSection(name.as_ptr()) };
    }

    #[cfg(target_os = "android")]
    fn on_exit(&self, _id: &Id, _ctx: Context<'_, S>) {
        // SAFETY: closes the section `on_enter` opened on this thread.
        unsafe { android::ATrace_endSection() };
    }
}

#[cfg(target_os = "android")]
mod android {
    use std::ffi::c_char;

    #[link(name = "android")]
    unsafe extern "C" {
        pub fn ATrace_beginSection(name: *const c_char);
        pub fn ATrace_endSection();
    }
}

#[cfg(target_os = "android")]
fn logcat(level: &str, text: &str) {
    use android_log_sys::{__android_log_write, LogPriority};
    let priority = match level {
        "trace" => LogPriority::VERBOSE,
        "debug" => LogPriority::DEBUG,
        "warn" => LogPriority::WARN,
        "error" => LogPriority::ERROR,
        _ => LogPriority::INFO,
    };
    let text = std::ffi::CString::new(text.replace('\0', " ")).unwrap_or_default();
    // SAFETY: both pointers are valid C strings for the call.
    unsafe { __android_log_write(priority as _, c"vitapmate".as_ptr(), text.as_ptr()) };
}

#[cfg(not(target_os = "android"))]
fn logcat(_level: &str, _text: &str) {}
