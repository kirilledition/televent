<objective>
Add comprehensive CalDAV request/response logging to the API server to enable debugging of client compatibility issues.

Currently, CalDAV clients like Thunderbird fail to display events despite curl/cadaver working correctly. Without detailed logging, it's impossible to diagnose what requests Thunderbird sends and how our responses differ from what it expects.
</objective>

<context>
Read the project conventions first:
@CLAUDE.md

Current CalDAV implementation files:
@crates/api/src/routes/caldav.rs - CalDAV route handlers
@crates/api/src/routes/caldav_xml.rs - XML response generation
@crates/api/src/middleware/caldav_auth.rs - Authentication middleware

Known situation from @docs/MVP_BLOCKERS_AND_PLAN.md:
- PROPFIND, REPORT, GET all return correct data when tested with curl
- Thunderbird subscribes but shows "calendar unavailable"
- No current CalDAV-specific logging exists
</context>

<requirements>
1. Add tracing-based logging for ALL CalDAV requests:
   - HTTP method and full URI path
   - All request headers (especially Depth, Content-Type, Authorization presence)
   - Full request body for PROPFIND and REPORT requests
   - Response status code
   - Full response body (truncated if >10KB)
   - Request duration

2. Log at appropriate levels:
   - INFO: Method, path, status, duration
   - DEBUG: Headers and body content
   - TRACE: Full untruncated bodies (for development only)

3. Use structured logging with fields:
   ```rust
   tracing::info!(
       method = %method,
       path = %uri,
       status = %status,
       duration_ms = %duration.as_millis(),
       "CalDAV request"
   );
   ```

4. Create a CalDAV-specific tracing span for request correlation

5. Add environment variable to enable verbose CalDAV logging:
   - `CALDAV_DEBUG=1` enables DEBUG level for caldav module
   - `CALDAV_DEBUG=2` enables TRACE level
</requirements>

<implementation>
Create or modify these files:

1. `crates/api/src/middleware/caldav_logging.rs` (new file)
   - Axum middleware/layer that wraps CalDAV routes
   - Capture request body before handlers consume it
   - Capture response body after handlers produce it
   - Use tower's ServiceBuilder pattern

2. `crates/api/src/routes/caldav.rs`
   - Add the logging layer to CalDAV router
   - Ensure middleware ordering: auth -> logging -> handlers

3. `crates/api/src/lib.rs` or main router setup
   - Configure tracing subscriber to respect CALDAV_DEBUG env var
   - Add caldav module to filter directives

Constraints (WHY these matter):
- Use `tracing` not `log` - project standard, enables structured queries
- Never log Authorization header values - security risk
- Truncate large bodies - prevent log storage exhaustion
- Use spans not just events - enables request correlation in async context
</implementation>

<output>
Files to create/modify:
- `./crates/api/src/middleware/caldav_logging.rs` - New logging middleware
- `./crates/api/src/middleware/mod.rs` - Export new module
- `./crates/api/src/routes/caldav.rs` - Add middleware to router

After implementation, test with:
```bash
CALDAV_DEBUG=1 cargo run --package api
# In another terminal:
curl -X PROPFIND http://localhost:3000/caldav/user/calendars/default \
  -H "Depth: 1" \
  -u "telegram_id:device_password"
```
</output>

<verification>
Before completing, verify:
1. CalDAV requests produce structured log output at INFO level
2. Setting CALDAV_DEBUG=1 shows request/response bodies
3. Authorization header values are NOT logged (check for credential leaks)
4. Log output includes timing information
5. Project compiles without warnings: `cargo clippy --package api`
</verification>

<success_criteria>
- All CalDAV requests produce structured logs
- Request and response bodies visible at DEBUG level
- No credentials in logs
- Middleware integrates cleanly with existing auth middleware
- Ready to diagnose Thunderbird compatibility issues
</success_criteria>
