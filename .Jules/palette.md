## 2024-05-23 - Smart Duration Selection
**Learning:** Flat lists with > 50 options (like minute-by-minute duration pickers) are overwhelming. Grouping options by scale (Minutes vs Hours) and using variable granularity (5m for short, 15m/30m for long) significantly improves scanability without sacrificing utility.
**Action:** Always group large select lists with `<optgroup>` and consider non-linear scales for range-based inputs.

## 2024-05-22 - Inconsistent Date/Time Pickers
**Learning:** The application uses two different patterns for event creation and editing. `CreateEvent` uses a custom scroll-based picker, while `EventForm` (used for editing) uses standard HTML5 inputs.
**Action:** When unifying UI or adding features, consider which pattern to standardize on. The custom picker is more touch-friendly but less accessible than native inputs.
