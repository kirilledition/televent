## 2024-05-22 - Verifying Loading States with Playwright Sync
**Learning:** Sync Playwright request interception blocks the main thread if you use `time.sleep()`. To verify UI state *during* a pending request, use a list to hold the `route` object and assert in the main loop before fulfilling.
**Action:** Use the hold-and-assert pattern for all future loading state verifications.
