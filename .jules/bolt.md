## 2025-02-18 - [Streaming CalDAV XML Generation]
**Learning:** Found an anti-pattern in CalDAV response generation where all events were first converted to iCal strings and stored in memory (Vec + HashMap) before XML generation. This caused double iteration and unnecessary allocations.
**Action:** Always prefer streaming data generation for large responses. When generating XML/JSON lists, convert items one-by-one inside the writer loop rather than pre-calculating them in bulk.
