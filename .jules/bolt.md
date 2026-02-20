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

## 2025-02-24 - [Vectorized String Processing in FoldedWriter]
**Learning:** Iterating over strings using `chars()` to process character-by-character (e.g., for escaping or folding) is significantly slower than block processing using `push_str` and vectorized search (`bytes().position()`).
**Insight:** For mostly-ASCII content (like iCalendar properties), finding special characters using byte-search and copying chunks is ~46% faster (13.47µs -> 7.28µs per event) than decoding UTF-8 characters one by one.
**Action:** When implementing escaping/folding logic, prefer chunked processing with `str::as_bytes()` and `push_str()` over `chars()` iteration.
