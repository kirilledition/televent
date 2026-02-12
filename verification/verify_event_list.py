from playwright.sync_api import sync_playwright

def run(playwright):
    browser = playwright.chromium.launch()
    page = browser.new_page()

    try:
        url = "http://localhost:3000/app"
        print(f"Navigating to {url}")
        page.goto(url)

        print("Waiting for page load...")
        page.wait_for_load_state("networkidle")

        # Verify Calendar heading
        heading = page.get_by_role("heading", name="Calendar")
        if heading.is_visible():
            print("Calendar heading found.")
        else:
            print("Calendar heading NOT found.")

        # Verify Events
        if page.get_by_text("Team Meeting").is_visible():
             print("Event 'Team Meeting' found.")
        else:
             print("Event 'Team Meeting' NOT found.")

        if page.get_by_text("Today").is_visible():
             print("Group header 'Today' found.")
        else:
             print("Group header 'Today' NOT found.")

        # Verify sorted order: 'Today' should come before 'Tomorrow'
        content = page.content()
        today_idx = content.find("Today")
        tomorrow_idx = content.find("Tomorrow")
        if today_idx != -1 and tomorrow_idx != -1 and today_idx < tomorrow_idx:
             print("Events are correctly sorted (Today before Tomorrow).")
        else:
             print("Events sorting issue or not found.")

        print("Taking screenshot...")
        page.screenshot(path="verification/event_list.png", full_page=True)
        print("Screenshot saved to verification/event_list.png")

    except Exception as e:
        print(f"Error: {e}")
    finally:
        browser.close()

with sync_playwright() as playwright:
    run(playwright)
