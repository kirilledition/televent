-- Create Enums
CREATE TYPE outbox_message_type AS ENUM ('email', 'telegram_notification', 'calendar_invite');
CREATE TYPE audit_action AS ENUM (
    'event_created', 'event_updated', 'event_deleted',
    'calendar_created', 'calendar_updated', 'calendar_deleted',
    'data_exported', 'account_deleted'
);
CREATE TYPE audit_entity_type AS ENUM ('event', 'calendar', 'user');

-- Alter outbox_messages table
ALTER TABLE outbox_messages
    ALTER COLUMN message_type TYPE outbox_message_type
    USING message_type::outbox_message_type;

-- Alter audit_log table
ALTER TABLE audit_log
    ALTER COLUMN action TYPE audit_action
    USING action::audit_action;

ALTER TABLE audit_log
    ALTER COLUMN entity_type TYPE audit_entity_type
    USING entity_type::audit_entity_type;
