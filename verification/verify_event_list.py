
from playwright.sync_api import sync_playwright

def verify_event_list():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        # Navigate to the page
        page.goto("http://localhost:3000/app")

        # Wait for the event list to load
        page.wait_for_selector("text=Today")

        # Verify "Today" is visible
        assert page.is_visible("text=Today")

        # Verify "Tomorrow" is visible
        assert page.is_visible("text=Tomorrow")

        # Verify dummy event content
        assert page.is_visible("text=Team Meeting")
        assert page.is_visible("text=Lunch with Client")
        assert page.is_visible("text=Code Review")

        # Take a screenshot
        page.screenshot(path="verification/verification.png")
        print("Verification successful! Screenshot saved to verification/verification.png")

        browser.close()

if __name__ == "__main__":
    verify_event_list()
