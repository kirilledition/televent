-- Create audit triggers for automatic logging
-- Tracks event changes for GDPR compliance

-- Function to log event changes
CREATE OR REPLACE FUNCTION log_event_change()
RETURNS TRIGGER AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        SELECT c.user_id, 'event_created', 'event', NEW.id
        FROM calendars c
        WHERE c.id = NEW.calendar_id;
        RETURN NEW;
    ELSIF (TG_OP = 'UPDATE') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        SELECT c.user_id, 'event_updated', 'event', NEW.id
        FROM calendars c
        WHERE c.id = NEW.calendar_id;
        RETURN NEW;
    ELSIF (TG_OP = 'DELETE') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        SELECT c.user_id, 'event_deleted', 'event', OLD.id
        FROM calendars c
        WHERE c.id = OLD.calendar_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Trigger for event changes
CREATE TRIGGER events_audit_trigger
    AFTER INSERT OR UPDATE OR DELETE ON events
    FOR EACH ROW
    EXECUTE FUNCTION log_event_change();

-- Function to log calendar changes
CREATE OR REPLACE FUNCTION log_calendar_change()
RETURNS TRIGGER AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        VALUES (NEW.user_id, 'calendar_created', 'calendar', NEW.id);
        RETURN NEW;
    ELSIF (TG_OP = 'UPDATE') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        VALUES (NEW.user_id, 'calendar_updated', 'calendar', NEW.id);
        RETURN NEW;
    ELSIF (TG_OP = 'DELETE') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        VALUES (OLD.user_id, 'calendar_deleted', 'calendar', OLD.id);
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Trigger for calendar changes
CREATE TRIGGER calendars_audit_trigger
    AFTER INSERT OR UPDATE OR DELETE ON calendars
    FOR EACH ROW
    EXECUTE FUNCTION log_calendar_change();

-- Comments
COMMENT ON FUNCTION log_event_change() IS 'Automatically logs event changes to audit_log for GDPR compliance';
COMMENT ON FUNCTION log_calendar_change() IS 'Automatically logs calendar changes to audit_log for GDPR compliance';
