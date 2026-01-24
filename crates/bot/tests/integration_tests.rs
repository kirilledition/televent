//! Integration tests for bot dispatcher using teloxide_tests

use bot::{build_handler_tree, db::BotDb};
use sqlx::PgPool;
use teloxide_tests::{MockBot, MockMessageText};
use teloxide::dptree::deps;
use chrono::{Utc, Duration};

/// Test that /start command gets routed correctly and creates a user
#[sqlx::test(migrations = "../../migrations")]
async fn test_dispatcher_start_command(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Create a mocked /start command message
    let mock_message = MockMessageText::new().text("/start").from().username("testuser").build();

    // Create mock bot with our handler tree
    let mut bot = MockBot::new(mock_message, build_handler_tree());

    // Add database dependency
    bot.dependencies(deps![db.clone()]);

    // Dispatch the update
    bot.dispatch().await;

    // Verify response
    let responses = bot.get_responses();
    let message = responses
        .sent_messages_text
        .last()
        .expect("No sent messages detected");

    assert!(message.message.text().unwrap().contains("Welcome"));

    // Verify user was created in database
    let telegram_id = 1; // Default mock user ID
    let user = sqlx::query!("SELECT * FROM users WHERE telegram_id = $1", telegram_id)
        .fetch_one(&pool)
        .await;

    assert!(user.is_ok());
}

/// Test that /help command gets routed correctly
#[sqlx::test(migrations = "../../migrations")]
async fn test_dispatcher_help_command(pool: PgPool) {
    let db = BotDb::new(pool);

    let mock_message = MockMessageText::new().text("/help");
    let mut bot = MockBot::new(mock_message, build_handler_tree());
    bot.dependencies(deps![db]);

    bot.dispatch().await;

    let responses = bot.get_responses();
    let message = responses
        .sent_messages_text
        .last()
        .expect("No sent messages detected");

    assert!(message.message.text().unwrap().contains("Available commands"));
}

/// Test that /list command gets routed correctly
#[sqlx::test(migrations = "../../migrations")]
async fn test_dispatcher_list_command(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Setup: create a user first
    let telegram_id = 1;
    db.ensure_user_setup(telegram_id, Some("listuser"))
        .await
        .unwrap();

    let mock_message = MockMessageText::new()
        .text("/list")
        .from().username("listuser").build();
    let mut bot = MockBot::new(mock_message, build_handler_tree());
    bot.dependencies(deps![db]);

    bot.dispatch().await;

    // Should send a message (even if no events)
    let responses = bot.get_responses();
    assert!(
        !responses.sent_messages_text.is_empty(),
        "Expected at least one message"
    );
}

/// Test text message routing to event creation
#[sqlx::test(migrations = "../../migrations")]
async fn test_dispatcher_text_message_event_creation(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Setup: create a user first
    let telegram_id = 1;
    db.ensure_user_setup(telegram_id, Some("eventuser"))
        .await
        .unwrap();

    // Multi-line event format
    let event_text = "Team Meeting\nnext monday at 2pm\n60\nDiscuss Q1 roadmap";

    let mock_message = MockMessageText::new()
        .text(event_text)
        .from().username("eventuser").build();
    let mut bot = MockBot::new(mock_message, build_handler_tree());
    bot.dependencies(deps![db.clone()]);

    bot.dispatch().await;

    let responses = bot.get_responses();
    let message = responses
        .sent_messages_text
        .last()
        .expect("No sent messages detected");

    // Should confirm event creation
    assert!(
        message.message.text().unwrap().contains("Event created")
            || message.message.text().unwrap().contains("created")
    );

    // Verify event was created
    let now = Utc::now();
    let events = db.get_events_for_user(telegram_id, now - Duration::days(1), now + Duration::days(365)).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].summary, "Team Meeting");
}

/// Test invalid text message handling
#[sqlx::test(migrations = "../../migrations")]
async fn test_dispatcher_invalid_text_message(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Setup user
    let telegram_id = 1;
    db.ensure_user_setup(telegram_id, Some("testuser"))
        .await
        .unwrap();

    // Single line - invalid event format
    let mock_message = MockMessageText::new()
        .text("just some random text")
        .from().username("testuser").build();
    let mut bot = MockBot::new(mock_message, build_handler_tree());
    bot.dependencies(deps![db]);

    bot.dispatch().await;

    let responses = bot.get_responses();
    let message = responses
        .sent_messages_text
        .last()
        .expect("No sent messages detected");

    // Should send error message
    assert!(
        message.message.text().unwrap().contains("format")
            || message.message.text().unwrap().contains("error")
            || message.message.text().unwrap().contains("invalid")
    );
}

/// Test /device command routing
#[sqlx::test(migrations = "../../migrations")]
async fn test_dispatcher_device_command(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Setup user
    let telegram_id = 1;
    db.ensure_user_setup(telegram_id, Some("deviceuser"))
        .await
        .unwrap();

    let mock_message = MockMessageText::new()
        .text("/device")
        .from().username("deviceuser").build();
    let mut bot = MockBot::new(mock_message, build_handler_tree());
    bot.dependencies(deps![db]);

    bot.dispatch().await;

    let responses = bot.get_responses();
    assert!(
        !responses.sent_messages_text.is_empty(),
        "Expected device help message"
    );
}

/// Test /export command routing
#[sqlx::test(migrations = "../../migrations")]
async fn test_dispatcher_export_command(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Setup user and event
    let telegram_id = 1;
    let user = db
        .ensure_user_setup(telegram_id, Some("exportuser"))
        .await
        .unwrap();

    let mock_message = MockMessageText::new()
        .text("/export")
        .from().username("exportuser").build();
    let mut bot = MockBot::new(mock_message, build_handler_tree());
    bot.dependencies(deps![db]);

    bot.dispatch().await;

    let responses = bot.get_responses();
    assert!(
        !responses.sent_messages_text.is_empty(),
        "Expected export response"
    );
}

/// Test multiple sequential commands
#[sqlx::test(migrations = "../../migrations")]
async fn test_dispatcher_multiple_commands(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // First command: /start
    let mock_message1 = MockMessageText::new().text("/start").from().username("multiuser").build();
    let mut bot = MockBot::new(mock_message1, build_handler_tree());
    bot.dependencies(deps![db.clone()]);
    bot.dispatch().await;

    // Second command: /help (reuse bot with new update)
    bot.update(MockMessageText::new().text("/help").from().username("multiuser").build());
    bot.dispatch().await;

    // Third command: /list
    bot.update(MockMessageText::new().text("/list").from().username("multiuser").build());
    bot.dispatch().await;

    let responses = bot.get_responses();

    // Should have responses from all three commands
    assert!(
        responses.sent_messages_text.len() >= 3,
        "Expected at least 3 messages from sequential commands"
    );
}
