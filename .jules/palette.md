## 2026-01-25 - Form Label Accessibility Gap
**Learning:** Found multiple form inputs using `<div>` wrappers for labels without `htmlFor`/`id` association, which breaks screen reader support and click-to-focus behavior. This seems to be a common pattern in the initial codebase.
**Action:** When auditing forms, immediately check for `htmlFor` attributes on labels and corresponding `id`s on inputs.
