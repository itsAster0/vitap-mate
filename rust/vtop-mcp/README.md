# vtop-mcp

A read-only MCP server for VTOP. It runs `vtop-core` against a session it
gets from `vtop-bridge`, so it never sees a password.

Students create an access key in VITAP Mate → Settings → Connected apps and
add the MCP URL it shows (the key is in the path) to their agent:

```
https://vtop-mcp-production.up.railway.app/mcp/vtm_…
```

`Authorization: Bearer vtm_…` on `/mcp` works too.

Transport is streamable HTTP, stateless, JSON responses. Keys are checked
against the bridge (`/v1/whoami`, cached 5 minutes). When VTOP rejects a
session, the server asks the bridge to expire it and retries once.

## Tools

`get_semesters`, `get_attendance`, `get_full_attendance`,
`get_timetable`, `get_marks`, `get_exam_schedule`, `get_grades`,
`get_grade_details`, `get_grade_history`, `get_biometric`.

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
