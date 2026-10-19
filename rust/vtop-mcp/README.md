# vtop-mcp

The MCP tools for VTOP, as a library: the engine (`vtop-bridge`) serves
them at `/mcp`, and the edge sends `/mcp` on to it. Every tool is read-only
except `get_session`, which hands a signed-in session to a browser. It runs
`vtop-core` against a session it gets from the edge with the caller's own
key, so it never sees a password.

Students create an access key in VITAP Mate → Settings → Connected apps and
add the MCP URL to their agent, with the key in the `Authorization` header:

```
URL:    https://vmate-api.aster0.dev/mcp
Header: Authorization: Bearer vtm_…
```

For Claude Code:

```sh
claude mcp add --transport http vtop https://vmate-api.aster0.dev/mcp \
  --header "Authorization: Bearer vtm_…"
```

The key is only read from the header; a key in the URL is not accepted.

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
| `get_academic_calendar` | `semester_id` | calendar entries sorted by date |
| `get_courses` | `semester_id` | course-page courses: id, code, title, type |
| `get_course_classes` | `semester_id`, `course_id` | classes: class_id, erp_id, slot, faculty |
| `get_course_detail` | `semester_id`, `erp_id`, `class_id` | lecture plan, syllabus, course plan |
| `get_general_outing` | — | general outing form and records |
| `get_weekend_outing` | — | weekend outing form and records |
| `get_session` | — | a live VTOP login for a browser: `url`, `cookies` (Playwright/Puppeteer shape), `cookie_header`. Not read-only. |

## Configuration

None of its own: the engine mounts it when `EDGE_URL` is set, and gets
sessions from that edge.

## Test

```sh
cd rust
cargo test -p vtop-mcp
```
