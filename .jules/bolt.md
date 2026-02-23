## 2025-02-18 - [Streaming CalDAV XML Generation]
**Learning:** Found an anti-pattern in CalDAV response generation where all events were first converted to iCal strings and stored in memory (Vec + HashMap) before XML generation. This caused double iteration and unnecessary allocations.
**Action:** Always prefer streaming data generation for large responses. When generating XML/JSON lists, convert items one-by-one inside the writer loop rather than pre-calculating them in bulk.

## 2025-02-19 - [Allocation-Free ETag Generation]
**Learning:** `to_rfc3339()` and `to_string()` on Chrono types allocate new strings, which is expensive in hot paths like ETag generation.
**Action:** Use `timestamp().to_be_bytes()` (for DateTime) and `num_days_from_ce().to_be_bytes()` (for NaiveDate) to hash the raw numeric data directly, eliminating intermediate string allocations.

## 2025-02-21 - [Buffer Reuse Anti-Pattern]
**Learning:** Attempted to optimize CalDAV XML generation by reusing a single `String` buffer (passed as `&mut String`) instead of creating a local `String::with_capacity(128)` inside the loop. Benchmarks showed this was ~40-80% SLOWER (23ms -> 38ms).
**Insight:** For small short-lived strings, the allocator is extremely optimized. Clearing and writing to a reused mutable string reference might introduce overheads (checks, dereferences) or prevent compiler optimizations (like putting the buffer on stack or registers) that outweigh the allocation cost.
**Action:** Don't assume buffer reuse is always faster. Measure! For small buffers, stack/local allocation might be faster.

## 2025-02-24 - [Avoid Intermediate String Allocations for Date Formatting]
**Learning:** `chrono::DateTime::format(...).to_string()` allocates a new String. In hot loops (like iCalendar generation), this adds significant overhead.
**Action:** Use `write!(buf, "{}", date.format(...))` to write directly to the destination buffer, bypassing the intermediate allocation. For known safe fields (short, no escaping needed), skipping general-purpose folding logic also yields gains (~22% speedup).

## 2025-03-03 - [Consuming Parsed Structs to Reduce Allocations]
**Learning:** `ical` crate parser produces owned `String` fields. Converting these to our internal model by iterating `&IcalEvent` forces cloning every string. By consuming `IcalEvent` (passed by value), we can move the strings directly or return them without allocation in the common case (no escaping needed).
**Action:** When mapping from a parsed owned structure to another, consume the source structure if possible. If some fields need special handling (like `ATTENDEE` properties processed separately), partition or extract them before consumption to avoid cloning.
