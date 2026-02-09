//! Integration tests for bot dispatcher using teloxide_tests

use bot::{build_handler_tree, db::BotDb};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use teloxide::dptree::deps;
use teloxide_tests::{MockBot, MockMessageText, MockPrivateChat, MockUser};

/// Test that /start command gets routed correctly and creates a user
#[sqlx::test(migrations = "../migrations")]
async fn test_dispatcher_start_command(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Create a mocked /start command message
    let mock_user = MockUser::new()
        .id(1)
        .first_name("Test".to_string())
        .username("testuser".to_string())
        .build();
    let mock_chat = MockPrivateChat::new()
        .id(1)
        .username("testuser".to_string())
        .build();
    let mock_message = MockMessageText::new()
        .text("/start")
        .from(mock_user)
        .chat(mock_chat);

    // Create mock bot with our handler tree
    let mut bot = MockBot::new(mock_message, build_handler_tree());

    // Add database dependency
    bot.dependencies(deps![db.clone()]);

    // Dispatch the update
    bot.dispatch().await;

    // Give a bit of time for DB tasks to settle
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify response
    let responses = bot.get_responses();
    let message = responses
        .sent_messages_text
        .last()
        .expect("No sent messages detected");

    assert!(message.message.text().unwrap().contains("Welcome"));
}

/// Test that /help command gets routed correctly
#[sqlx::test(migrations = "../migrations")]
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

    assert!(
        message
            .message
            .text()
            .unwrap()
            .contains("Televent Commands")
    );
}

/// Test that /list command gets routed correctly
#[sqlx::test(migrations = "../migrations")]
async fn test_dispatcher_list_command(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Setup: create a user first
    let telegram_id = 1;
    db.ensure_user_setup(telegram_id, Some("listuser"))
        .await
        .unwrap();

    let mock_user = MockUser::new()
        .id(1)
        .username("listuser".to_string())
        .build();
    let mock_chat = MockPrivateChat::new()
        .id(1)
        .username("listuser".to_string())
        .build();
    let mock_message = MockMessageText::new()
        .text("/list")
        .from(mock_user)
        .chat(mock_chat);
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
#[sqlx::test(migrations = "../migrations")]
async fn test_dispatcher_text_message_event_creation(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Setup: create a user first
    let telegram_id = 1;
    db.ensure_user_setup(telegram_id, Some("eventuser"))
        .await
        .unwrap();

    // Multi-line event format
    let event_text = "Team Meeting\nnext monday at 2pm\n60\nDiscuss Q1 roadmap";

    let mock_user = MockUser::new()
        .id(1)
        .username("eventuser".to_string())
        .build();
    let mock_chat = MockPrivateChat::new()
        .id(1)
        .username("eventuser".to_string())
        .build();
    let mock_message = MockMessageText::new()
        .text(event_text)
        .from(mock_user)
        .chat(mock_chat);
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
        message.message.text().unwrap().contains("Event Created!")
            || message.message.text().unwrap().contains("Created!")
    );

    // Verify event was created
    let now = Utc::now();
    let events = db
        .get_events_for_user(
            telegram_id,
            now - Duration::days(1),
            now + Duration::days(365),
        )
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].summary, "Team Meeting");
}

/// Test invalid text message handling
#[sqlx::test(migrations = "../migrations")]
async fn test_dispatcher_invalid_text_message(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Setup user
    let telegram_id = 1;
    db.ensure_user_setup(telegram_id, Some("testuser"))
        .await
        .unwrap();

    // Single line - invalid event format
    let mock_user = MockUser::new()
        .id(1)
        .username("testuser".to_string())
        .build();
    let mock_chat = MockPrivateChat::new()
        .id(1)
        .username("testuser".to_string())
        .build();
    let mock_message = MockMessageText::new()
        .text("Invalid Event\nNot a date")
        .from(mock_user)
        .chat(mock_chat);
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
#[sqlx::test(migrations = "../migrations")]
async fn test_dispatcher_device_command(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Setup user
    let telegram_id = 1;
    db.ensure_user_setup(telegram_id, Some("deviceuser"))
        .await
        .unwrap();

    let mock_user = MockUser::new()
        .id(1)
        .username("deviceuser".to_string())
        .build();
    let mock_chat = MockPrivateChat::new()
        .id(1)
        .username("deviceuser".to_string())
        .build();
    let mock_message = MockMessageText::new()
        .text("/device")
        .from(mock_user)
        .chat(mock_chat);
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
#[sqlx::test(migrations = "../migrations")]
async fn test_dispatcher_export_command(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    // Setup user and event
    let telegram_id = 1;
    let _user = db
        .ensure_user_setup(telegram_id, Some("exportuser"))
        .await
        .unwrap();

    let mock_user = MockUser::new()
        .id(1)
        .username("exportuser".to_string())
        .build();
    let mock_chat = MockPrivateChat::new()
        .id(1)
        .username("exportuser".to_string())
        .build();
    let mock_message = MockMessageText::new()
        .text("/export")
        .from(mock_user)
        .chat(mock_chat);
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
#[sqlx::test(migrations = "../migrations")]
async fn test_dispatcher_multiple_commands(pool: PgPool) {
    let db = BotDb::new(pool.clone());

    let mock_user = MockUser::new()
        .id(1)
        .username("multiuser".to_string())
        .build();
    let mock_chat = MockPrivateChat::new()
        .id(1)
        .username("multiuser".to_string())
        .build();

    let updates = vec![
        MockMessageText::new()
            .text("/start")
            .from(mock_user.clone())
            .chat(mock_chat.clone()),
        MockMessageText::new()
            .text("/help")
            .from(mock_user.clone())
            .chat(mock_chat.clone()),
        MockMessageText::new()
            .text("/list")
            .from(mock_user)
            .chat(mock_chat),
    ];

    let mut bot = MockBot::new(updates, build_handler_tree());
    bot.dependencies(deps![db]);
    bot.dispatch().await;

    let responses = bot.get_responses();

    // Should have responses from all three commands
    assert!(
        responses.sent_messages_text.len() >= 3,
        "Expected at least 3 messages from sequential commands"
    );
}
