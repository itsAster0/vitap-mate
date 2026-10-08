# vtop-mcp

A read-only MCP server for VTOP. It runs `vtop-core` against a session it
gets from `vtop-bridge`, so it never sees a password.

Students create an access key in VITAP Mate → Settings → Connected apps and
add the MCP URL it shows (the key is in the path) to their agent:

```
https://vtop-mcp.aster0.dev/mcp/vtm_…
```

`Authorization: Bearer vtm_…` on `/mcp` works too.

Transport is streamable HTTP, stateless, JSON responses. Keys are checked
against the bridge (`/v1/whoami`, cached 5 minutes). When VTOP rejects a
session, the server asks the bridge to expire it and retries once.

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

## Configuration

| Variable     | Default | Notes                                              |
| ------------ | ------- | -------------------------------------------------- |
| `BRIDGE_URL` | —       | required; on Railway the bridge's private address |
| `PORT`       | 8080    | listens on `[::]`                                  |

## Run and test

```sh
cd rust
cargo test -p vtop-mcp
BRIDGE_URL=http://localhost:8090 cargo run -p vtop-mcp
```

Railway builds it with Railpack from `rust/railpack.vtop-mcp.json`.
