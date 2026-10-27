# vtop-mcp

The MCP tools for VTOP, as a library: the engine (`vtop-bridge`) serves
them at `/mcp`, and the edge sends `/mcp` on to it. Every tool is read-only
except `get_session`, which hands a signed-in session to a browser, and the
outing tools that apply for or cancel a request. It runs
`vtop-core` against a session it gets from the edge with the caller's own
key, so it never sees a password.

Students create an access key in VITAP Mate → Settings → Connected apps and
add the MCP URL to their agent, with the key in the `Authorization` header:

```
URL:    https://vmate-api.aster0.dev/mcp
Header: Authorization: Bearer vtm_…
```

An agent that only takes a URL can carry the key in the path instead:

```
https://vmate-api.aster0.dev/mcp/vtm_…
```

For Claude Code:

```sh
claude mcp add --transport http vtop https://vmate-api.aster0.dev/mcp \
  --header "Authorization: Bearer vtm_…"
```

The header wins when both are sent. A URL with the key in it is as secret
as the key: it can show up in the agent's config and in request logs.

Transport is streamable HTTP, stateless, JSON responses. Keys are checked
against the edge (`/v1/whoami`, cached 5 minutes). When VTOP rejects a
session, the server asks the edge to expire it and retries once.

## Tools

| Tool | Arguments | Returns |
| --- | --- | --- |
| `whoami` | — | registration number and key label (bridge only, no VTOP call) |
| `get_semesters` | — | semester ids and names; call first |
| `get_attendance` | `semester_id` | attendance summary per course |
| `get_full_attendance` | `semester_id`, `course_id`, `course_type` | class-by-class attendance |
| `get_timetable` | `semester_id` | weekly timetable |
| `get_marks` | `semester_id` | internal marks |
| `get_exam_schedule` | `semester_id` | exam dates, slots, venues, seats |
| `get_grades` | `semester_id` | final grades |
| `get_grade_details` | `semester_id`, `course_id` | mark breakdown behind a grade |
| `get_grade_history` | — | all grades and CGPA |
| `get_biometric` | `date` (DD/MM/YYYY) | entry log for a day |
| `get_biometric_range` | `from`, `to` (DD/MM/YYYY, ≤ 14 days) | entry log for each day |
| `get_profile` | — | the student's profile (no photo, Aadhaar masked) |
| `get_mentor` | — | mentor details and this week's messages |
| `get_academic_calendar` | `semester_id` | calendar entries sorted by date |
| `get_courses` | `semester_id` | course-page courses: id, code, title, type |
| `get_course_classes` | `semester_id`, `course_id` | classes: class_id, erp_id, slot, faculty |
| `get_course_detail` | `semester_id`, `erp_id`, `class_id` | lecture plan, syllabus, course plan |
| `get_course_file` | `path` (from `get_course_detail`) | the file, as an embedded resource (≤ 10 MB) |
| `get_course_plan` | `semester_id`, `class_id` | the course plan sheet, as an embedded resource |
| `get_general_outing` | — | general outing form and records |
| `get_weekend_outing` | — | weekend outing form and records |
| `apply_general_outing` | `place`, `purpose`, `out_date`, `out_time`, `in_date`, `in_time`, `confirmation?` | a preview and confirmation code, or VTOP's answer. Not read-only. |
| `apply_weekend_outing` | `place`, `purpose`, `date`, `time_slot`, `contact_number`, `confirmation?` | a preview and confirmation code, or VTOP's answer. Not read-only. |
| `cancel_outing` | `kind`, `cancel_id`, `confirmation?` | the request and a confirmation code, or VTOP's answer. Destructive. |
| `get_outing_pass` | `kind`, `pass_id` | an accepted outing's PDF pass, as an embedded resource |
| `get_session` | — | a live VTOP login for a browser: `url`, `cookies` (Playwright/Puppeteer shape), `cookie_header`. Not read-only. |

Every tool carries MCP annotations: the read-only ones say `readOnlyHint`,
so clients can run them without asking; `get_session` and the outing tools
that apply or cancel do not.

## Prompts

Ready-made requests that clients offer as slash commands (in Claude Code,
`/mcp__vtop__weekly-briefing` and so on). They are text only: they tell the
agent which tools to use and call nothing themselves.

| Prompt | Arguments | Asks for |
| --- | --- | --- |
| `weekly-briefing` | — | classes each day, attendance at risk, exams, mentor messages, outings |
| `can-i-skip` | `day?` (default tomorrow) | whether missing that day keeps every course at 75% |
| `marks-needed` | `course?` | the FAT mark needed to pass and for each grade |
| `exam-prep` | `course?` | the next exam, what it covers, and a study plan |
| `apply-outing` | `kind?` | an outing application, step by step, confirmed before sending |

## Outing permission

`apply_general_outing`, `apply_weekend_outing` and `cancel_outing` work
only with a key the student has allowed to act on outings: a switch when
creating the key in VITAP Mate → Settings → Connected apps, or by tapping
the key there later. It is off by default, and keys made before it existed
start off. Without it those tools answer `outings_off` before anything is
sent to VTOP; reading outings and `get_outing_pass` still work.

The edge keeps the flag on the key (`outings`), returns it from
`/v1/whoami` as `keyOutings`, and the app sets it with `/v1/keys` (on
create) and `/v1/keys/outings` (`{ id, outings }`). The tools ask
`/v1/whoami` on every outing action rather than using the five-minute key
cache, so turning it off takes effect at once. `whoami` shows it as
`outings`.

## Confirming outing changes

Applying for and cancelling an outing take two calls. Without
`confirmation`, the tool checks the request against a fresh copy of VTOP's
form (the same checks the real submit makes, plus the date limits the app's
date pickers enforce) and returns the request as VTOP would get it, with a
confirmation code. Nothing is sent. Called again with the same arguments
and that code, it sends the request.

The code is a hash of the caller's key, the action and the exact
arguments, with an expiry ten minutes out, so a code only works for the
request it was given for. The server keeps no state, so the code shows the
agent saw the preview, not that the student said yes: the tool descriptions
and instructions tell the agent to ask first. Before sending, the tool
checks VTOP's list again and refuses a request that is already there, so a
repeated code or a retry cannot file it twice.

## Configuration

None of its own: the engine mounts it when `EDGE_URL` is set, and gets
sessions from that edge.

## Test

```sh
cd rust
cargo test -p vtop-mcp
```
