-- Remove database support for deleted entities

-- 1. Remove deletion trigger from events
DROP TRIGGER IF EXISTS events_capture_deletion ON events;
DROP FUNCTION IF EXISTS capture_event_deletion();

-- 2. Drop deleted_events table
DROP TABLE IF EXISTS deleted_events;
