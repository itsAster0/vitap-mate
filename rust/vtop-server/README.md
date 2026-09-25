# vtop-server

An HTTP service that fetches and parses VTOP pages on behalf of the app. It
runs the same `vtop-core` code the app runs on the device and answers with the
same JSON shapes (`vtop-core/src/types.rs`, serialised by serde: snake_case
keys, `update_time` as a number, `ClassKind` as `"Theory"`/`"Lab"`).

The server is stateless. Callers send their VTOP session with every request.
The only thing kept in memory is an optional short-lived cache of validated
clients, keyed by a SHA-256 of the cookie header. Cookies, passwords, OTPs
and API keys are never logged: the access log records method, path, status,
latency and the index of the API key.

## Run it

```sh
cd rust
# with a key (recommended for anything reachable from the internet)
VTOP_SERVER_API_KEYS="$(openssl rand -base64 32)" cargo run --profile server -p vtop-server
# open, for local testing
cargo run --profile server -p vtop-server
```

or with Docker from the repository root:

```sh
docker build -f rust/vtop-server/Dockerfile -t vtop-server .
docker run -p 8080:8080 -e VTOP_SERVER_API_KEYS=... vtop-server
```

### Railway

The Railway service (`vtop-server`) is defined in `.railway/railway.ts` at the
repository root, applied with `railway config plan` / `railway config apply`.
It sets Root Directory to `/rust` (without it the build sees the Flutter repo
and finds no `Cargo.toml`) and builds with Railpack, which reads
`rust/railpack.json`: a current Rust and the `server` profile (Railpack's
default `--release` would use the app library's size-optimised,
`panic = "abort"` profile). Set in the dashboard:

- `VTOP_SERVER_API_KEYS` (a public Railway URL should not run open). Railway supplies `PORT`, and `/health` is the health
check.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `VTOP_SERVER_API_KEYS` | (none: open) | Comma-separated accepted keys, each at least 8 characters. Generate with `openssl rand -base64 32`. Unset or empty means **open mode**: no `X-API-Key` check, and all callers share one rate-limit bucket. |
| `PORT` | `8080` | Port to listen on (all interfaces). Ignored when `VTOP_SERVER_BIND` is set. |
| `VTOP_SERVER_BIND` | – | Explicit `host:port`, e.g. `127.0.0.1:8080`. |
| `VTOP_SERVER_REQUEST_TIMEOUT_SECS` | `45` | Limit for a whole data request, VTOP round trips included. |
| `VTOP_SERVER_LOGIN_TIMEOUT_SECS` | `120` | Limit for login and OTP requests (CAPTCHA retries are slow). |
| `VTOP_UPSTREAM_TIMEOUT_SECS` | `30` | Limit for one HTTP request to VTOP. |
| `VTOP_CONNECT_TIMEOUT_SECS` | `10` | TCP/TLS connect limit for VTOP. |
| `VTOP_SERVER_RATE_LIMIT_PER_MINUTE` | `120` | Requests per minute per API key (token bucket, bursts up to the same number). `0` disables. |
| `VTOP_SERVER_SESSION_CACHE_TTL_SECS` | `300` | How long a validated session stays cached. `0` disables the cache. |
| `VTOP_SERVER_SESSION_CACHE_MAX` | `5000` | Maximum cached sessions. |
| `VTOP_SERVER_CONCURRENCY` | `8` | VTOP requests one `/v1/refresh` runs at once (pages, then course details). |
| `VTOP_SERVER_GMAIL_WAIT_SECS` | `45` | How long a login waits for VTOP's OTP email when the caller sent a Gmail token. |
| `VTOP_SERVER_ALLOW_LOGIN` | `true` | Set `false` to refuse `/v1/auth/*`, so the server never sees passwords. |
| `VTOP_BASE_URL` | `https://vtop.vitap.ac.in` | VTOP origin. |
| `RUST_LOG` | `info` | Log filter, e.g. `info,rust.network=warn`. |

## API

Every `/v1` endpoint is `POST` with a JSON body and needs the `X-API-Key`
header when keys are configured. `GET /health` never needs one.

### Open mode

Without `VTOP_SERVER_API_KEYS` the server accepts everyone and logs a
warning at startup. That suits localhost or a private network. On a public
URL, set a key: stateless does not mean harmless. An open server lets anyone
use it as a proxy to VTOP (which can get your host's IP blocked), send VTOP
passwords to `/v1/auth/login`, and spend your CPU and hosting budget.

`session` is the caller's VTOP session:

```json
{ "cookies": "JSESSIONID=...; SERVERID=...",
  "csrf_token": "...", "registration_number": "..." }
```

`logged_in_at` (unix seconds of the last real login) comes back from the
login endpoints, so callers can re-login after a session age they choose.

Only `cookies` is required. With just a cookie, the server checks it against
VTOP once, then caches the result. Every successful data response also carries
an `X-Vtop-Session` header: the refreshed session as base64url JSON. Sending
that back in full lets the server skip the validation round trip entirely.

| Endpoint | Body besides `session` | Response |
| --- | --- | --- |
| `/v1/semesters` | – | `SemesterData` |
| `/v1/attendance` | `semester_id` | `AttendanceData` |
| `/v1/attendance/full` | `semester_id`, `course_id`, `course_type` | `FullAttendanceData` |
| `/v1/timetable` | `semester_id` | `TimetableData` |
| `/v1/marks` | `semester_id` | `MarksData` |
| `/v1/exam-schedule` | `semester_id` | `ExamScheduleData` |
| `/v1/grades` | `semester_id` | `GradeViewData` |
| `/v1/grades/details` | `semester_id`, `course_id` | `GradeDetailsData` |
| `/v1/grade-history` | – | `GradeHistoryData` |
| `/v1/biometric` | `date` (`DD/MM/YYYY`) | `BiometricData` |
| `/v1/refresh` | `semester_id`, optional `include` | several pages at once (below) |

### Batch refresh

`/v1/refresh` fetches several pages in one call, so a phone makes one round
trip instead of five. `include` picks from `semesters`, `attendance`,
`timetable`, `marks`, `exam_schedule`, `grades` (default: all of these) and
`full_attendance`, which must be asked for because it costs one VTOP request
per course. `full_attendance` returns a list of `FullAttendanceData`, one per
course on the attendance page (reusing that page when `attendance` is also
included); a course whose detail fails is left out so the caller can fetch it
alone. Each part
comes back as either `{"data": …}` or `{"error": {"code", "message"}}`:

```json
{ "attendance": { "data": { "records": [...], "semester_id": "...", "update_time": 0 } },
  "marks": { "error": { "code": "vtop_error", "message": "..." } } }
```

Pages are fetched one after another. VTOP serves one request per session at
a time, and a concurrent fetch measured slower (1030 ms against 737 ms for
five pages). An expired session fails the whole call with 401
`session_expired`.

Responses are gzip-compressed when the client sends `Accept-Encoding: gzip`
(the six-page refresh shrinks from about 24.6 KB to 3.2 KB). VTOP itself
ignores `Accept-Encoding` and only speaks HTTP/1.1, so pages from VTOP always
arrive uncompressed. A VTOP 5xx, timeout or refused connection is retried
once after 400 ms.

### Login (optional)

The server can sign in for a caller and hand the session back. It keeps
nothing, so the caller stores the returned `session` and sends it along from
then on. Turn this off with `VTOP_SERVER_ALLOW_LOGIN=false`.

| Endpoint | Body | Response |
| --- | --- | --- |
| `/v1/auth/login` | `username`, `password`, optional `gmail` | `{"status":"authenticated","session":…}` or `{"status":"otp_required","session":…,"message":…,"issued_at":…}` |
| `/v1/auth/otp` | `session` (from `otp_required`), `otp` | `{"status":"authenticated","session":…}` |
| `/v1/auth/otp/resend` | `session` | `{"status":"otp_sent","session":…}` |

### OTP from Gmail

A login can answer VTOP's emailed OTP by itself. Send a Gmail OAuth
**access token** (never the refresh token) with the login:

```json
{ "username": "...", "password": "...",
  "gmail": { "access_token": "...", "expires_at": 1790000000,
             "delete_after_reading": true } }
```

If VTOP asks for the OTP, the server polls the mailbox for the newest mail
from `noreply.sdc@vitap.ac.in` (up to `VTOP_SERVER_GMAIL_WAIT_SECS`), submits
the code, then trashes the email or marks it read. It uses the same Rust code
the app uses for its own auto-fetch. If no usable email arrives, the reply is
the usual `otp_required` plus a `gmail` field saying why:

| `gmail` | Meaning |
| --- | --- |
| `gmail_unauthorized` | The token expired (or expires before the wait ends) or was revoked: refresh it |
| `gmail_forbidden` | Gmail refused access: missing scope or Gmail API not enabled |
| `gmail_otp_not_found` | No OTP email arrived in time |
| `gmail_unavailable` | Gmail could not be reached |

The token is used only for that request and is never stored or logged. Send
one with at least a few minutes of validity left.

### Errors

```json
{ "error": { "code": "session_expired", "message": "Session has expired" } }
```

| Status | Codes |
| --- | --- |
| 400 | `bad_request`, `invalid_input` |
| 401 | `unauthorized` (API key), `session_expired`, `invalid_credentials`, `authentication_failed`, `otp_required` |
| 403 | `login_disabled` |
| 429 | `rate_limited` (with `Retry-After`) |
| 502 | `vtop_unreachable`, `vtop_error`, `vtop_unparseable` |
| 503 | `captcha_unavailable` |
| 504 | `timeout` |

On `session_expired`, sign in again (on the device, or through `/v1/auth/login`).

## Examples

```sh
BASE=http://localhost:8080
KEY=your-api-key
COOKIE='JSESSIONID=...; SERVERID=...'

curl -s $BASE/health

curl -s $BASE/v1/semesters \
  -H "X-API-Key: $KEY" -H 'Content-Type: application/json' \
  -d "{\"session\":{\"cookies\":\"$COOKIE\"}}"

curl -s $BASE/v1/attendance \
  -H "X-API-Key: $KEY" -H 'Content-Type: application/json' \
  -d "{\"session\":{\"cookies\":\"$COOKIE\"},\"semester_id\":\"AP2026271\"}"

curl -s $BASE/v1/biometric \
  -H "X-API-Key: $KEY" -H 'Content-Type: application/json' \
  -d "{\"session\":{\"cookies\":\"$COOKIE\"},\"date\":\"18/07/2026\"}"

# Login, then answer the OTP if VTOP asks for one
curl -s $BASE/v1/auth/login \
  -H "X-API-Key: $KEY" -H 'Content-Type: application/json' \
  -d '{"username":"22BCE0000","password":"..."}'
curl -s $BASE/v1/auth/otp \
  -H "X-API-Key: $KEY" -H 'Content-Type: application/json' \
  -d '{"session":{...session from the login reply...},"otp":"123456"}'
```

Keep API keys and cookies out of shell history in real use (e.g. read them
from a file).
