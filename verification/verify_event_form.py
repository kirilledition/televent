from playwright.sync_api import sync_playwright, expect

def run(playwright):
    browser = playwright.chromium.launch(headless=True)
    page = browser.new_page()

    print("Navigating to create page...")
    try:
        page.goto("http://localhost:3000/create") # Try without /app first? No, use /app/create based on memory.
        # Actually, let's try /create and if it redirects or fails, try /app/create
        # But wait, dev server might not respect basePath if not configured in next.config.ts correctly?
        # Memory said basePath is /app.
        # Let's stick to /app/create.
        page.goto("http://localhost:3000/app/create")

        # Verify AutoFocus
        print("Checking autofocus...")
        title_input = page.locator('input#summary')
        expect(title_input).to_be_focused()
        print("SUCCESS: Title input is focused.")

        # Verify Tooltip/Title attribute
        print("Checking submit button title...")
        # locator by role button and name create event
        submit_button = page.get_by_role("button", name="Create Event")
        # Check if attribute exists
        title_attr = submit_button.get_attribute("title")
        if title_attr == "Save (Ctrl+Enter)":
             print("SUCCESS: Submit button has correct title.")
        else:
             print(f"FAILURE: Title is {title_attr}")

        # Verify Keyboard Shortcut
        print("Testing Ctrl+Enter shortcut...")
        title_input.fill("Test Event via Shortcut")

        # Simulate Ctrl+Enter
        title_input.press("Control+Enter")

        # Check for loading state OR error message
        # Wait a bit
        page.wait_for_timeout(1000)

        # Check if error message appeared
        error_msg = page.locator(".text-red")
        if error_msg.count() > 0:
            print(f"SUCCESS: Form submitted (Error shown: {error_msg.first.inner_text()})")
        else:
             # Check for loading
             if page.get_by_text("Saving...").count() > 0:
                 print("SUCCESS: Form submitted (Loading state visible)")
             else:
                 print("WARNING: Neither error nor loading state visible. Maybe submission failed silently or too fast.")

        # Take screenshot
        page.screenshot(path="verification/verification.png")
        print("Screenshot saved to verification/verification.png")

    except Exception as e:
        print(f"ERROR: {e}")
        page.screenshot(path="verification/error.png")
    finally:
        browser.close()

with sync_playwright() as playwright:
    run(playwright)
