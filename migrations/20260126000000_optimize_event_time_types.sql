-- 1. Add explicit DATE columns for All-Day events
ALTER TABLE events
    ADD COLUMN start_date DATE,
    ADD COLUMN end_date DATE;

-- 2. Migrate existing data
-- Convert current "All Day" timestamps (stored as UTC midnight) to DATEs.
-- We use the event's stored timezone to correctly interpret "Midnight".
UPDATE events
SET
    start_date = (start AT TIME ZONE timezone)::date,
    end_date = ("end" AT TIME ZONE timezone)::date
WHERE is_all_day = true;

-- 3. Make old timestamp columns nullable (since All-Day events won't use them)
ALTER TABLE events
    ALTER COLUMN start DROP NOT NULL,
    ALTER COLUMN "end" DROP NOT NULL;

-- 4. Add "XOR" Check Constraint
-- Ensures a row is strictly EITHER a "Time Event" OR a "Date Event".
ALTER TABLE events
    ADD CONSTRAINT check_event_type_integrity CHECK (
        (is_all_day = true  AND start_date IS NOT NULL AND end_date IS NOT NULL AND start IS NULL AND "end" IS NULL) OR
        (is_all_day = false AND start_date IS NULL AND end_date IS NULL AND start IS NOT NULL AND "end" IS NOT NULL)
    );

-- 5. Create index for the new column
CREATE INDEX idx_events_start_date ON events(user_id, start_date);
