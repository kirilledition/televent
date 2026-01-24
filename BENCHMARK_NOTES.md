# Performance Optimization: List Events Pagination

## Issue
The `list_events` endpoint previously returned all events for a calendar (optionally within a time range) without any limit. This poses a scalability risk:
- **Memory Usage**: Loading thousands of events into memory can cause OOM kills.
- **Latency**: Serializing and transferring large JSON responses increases latency.
- **Database Load**: Large result sets put pressure on the database I/O and network.

## Optimization
We are implementing `limit` and `offset` pagination.
- Default limit: 100 events (if not specified).
- Client can specify `limit` and `offset`.

## Measurement Limitations
The current development environment does not have a running PostgreSQL instance, and the integration tests (`crates/api/tests/caldav_integration.rs`) fail due to missing `DATABASE_URL`. Therefore, we cannot run a live benchmark (e.g., using `criterion` or `cargo test`) to measure the exact latency improvement.

## Rationale for Improvement
Pagination is a standard database optimization pattern.
- **Complexity**: O(limit) instead of O(total_count) for data transfer and serialization.
- **Database Index**: The query uses `ORDER BY start`. With an index on `(calendar_id, start)`, accessing the first N rows is efficient.
