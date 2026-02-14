## 2024-05-22 - Inconsistent Date/Time Pickers
**Learning:** The application uses two different patterns for event creation and editing. `CreateEvent` uses a custom scroll-based picker, while `EventForm` (used for editing) uses standard HTML5 inputs.
**Action:** When unifying UI or adding features, consider which pattern to standardize on. The custom picker is more touch-friendly but less accessible than native inputs.
