## 2026-01-25 - Form Label Accessibility Gap
**Learning:** Found multiple form inputs using `<div>` wrappers for labels without `htmlFor`/`id` association, which breaks screen reader support and click-to-focus behavior. This seems to be a common pattern in the initial codebase.
**Action:** When auditing forms, immediately check for `htmlFor` attributes on labels and corresponding `id`s on inputs.

## 2026-02-01 - Inline Delete Confirmation
**Learning:** For list items on mobile-first interfaces, replacing the action button with inline confirm/cancel buttons is a superior pattern to modal dialogs. It keeps the context local and reduces friction while preventing accidental touches. The `var(--ctp-red)` color works well for destructive actions in this theme.
**Action:** Use this inline state pattern for other destructive list actions instead of full modals.
