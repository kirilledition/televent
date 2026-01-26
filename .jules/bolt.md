## 2025-02-18 - [Streaming CalDAV XML Generation]
**Learning:** Found an anti-pattern in CalDAV response generation where all events were first converted to iCal strings and stored in memory (Vec + HashMap) before XML generation. This caused double iteration and unnecessary allocations.
**Action:** Always prefer streaming data generation for large responses. When generating XML/JSON lists, convert items one-by-one inside the writer loop rather than pre-calculating them in bulk.

## 2025-02-19 - [Allocation-Free ETag Generation]
**Learning:** `to_rfc3339()` and `to_string()` on Chrono types allocate new strings, which is expensive in hot paths like ETag generation.
**Action:** Use `timestamp().to_be_bytes()` (for DateTime) and `num_days_from_ce().to_be_bytes()` (for NaiveDate) to hash the raw numeric data directly, eliminating intermediate string allocations.
