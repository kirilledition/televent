## 2024-05-22 - Verifying Loading States with Playwright Sync
**Learning:** Sync Playwright request interception blocks the main thread if you use `time.sleep()`. To verify UI state *during* a pending request, use a list to hold the `route` object and assert in the main loop before fulfilling.
**Action:** Use the hold-and-assert pattern for all future loading state verifications.

## 2026-02-04 - Context in Grid Interfaces
**Learning:** In grid-based date pickers, visual position provides context (row/col) that screen readers lack. Numbers like "1, 2, 3" are meaningless audibly.
**Action:** Always construct full labels (e.g., "January 1, 2024") for grid items to ensure context is available to all users.
