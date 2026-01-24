//! Event message parser
//!
//! Parses multi-line text messages into event data for creation.

use chrono::{DateTime, Duration, Local, Utc};
use chrono_english::{Dialect, parse_date_string};
use thiserror::Error;

/// Errors that can occur during event parsing
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Event title is required (line 1)")]
    MissingTitle,

    #[error("Date/time is required (line 2)")]
    MissingDateTime,

    #[error(
        "Could not parse date/time: {0}. Try formats like 'tomorrow 2pm', 'next Monday 10:00', or '2026-01-25 14:00'"
    )]
    InvalidDateTime(String),

    #[error("Duration must be a positive number of minutes")]
    InvalidDuration,

    #[error("Message must have at least 2 lines (title and date/time)")]
    TooFewLines,
}

/// A successfully parsed event ready for creation
#[derive(Debug, Clone)]
pub struct ParsedEvent {
    /// Event title/summary
    pub title: String,
    /// Event start time in UTC
    pub start: DateTime<Utc>,
    /// Duration in minutes (default: 60)
    pub duration_minutes: u32,
    /// Optional location
    pub location: Option<String>,
}

impl ParsedEvent {
    /// Calculate the end time based on start + duration
    pub fn end_time(&self) -> DateTime<Utc> {
        self.start + Duration::minutes(i64::from(self.duration_minutes))
    }
}

/// Parse a multi-line message into event data
///
/// Expected format:
/// ```text
/// Event Title
/// tomorrow at 2pm
/// 60
/// Conference Room A
/// ```
///
/// Lines:
/// 1. Event title (required)
/// 2. Date/time - natural language (required)
/// 3. Duration in minutes (optional, default: 60)
/// 4. Location (optional)
pub fn parse_event_message(text: &str) -> Result<ParsedEvent, ParseError> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();

    // Must have at least 2 lines (title and date)
    if lines.len() < 2 {
        return Err(ParseError::TooFewLines);
    }

    // Line 1: Title (required)
    let title = lines[0].to_string();
    if title.is_empty() {
        return Err(ParseError::MissingTitle);
    }

    // Line 2: Date/time (required)
    let datetime_str = lines[1];
    if datetime_str.is_empty() {
        return Err(ParseError::MissingDateTime);
    }

    // Parse datetime using chrono-english for natural language
    let start = parse_datetime(datetime_str)?;

    // Line 3: Duration in minutes (optional, default: 60)
    let duration_minutes = if lines.len() > 2 && !lines[2].is_empty() {
        lines[2]
            .parse::<u32>()
            .map_err(|_| ParseError::InvalidDuration)?
    } else {
        60
    };

    if duration_minutes == 0 {
        return Err(ParseError::InvalidDuration);
    }

    // Line 4: Location (optional)
    let location = if lines.len() > 3 && !lines[3].is_empty() {
        Some(lines[3].to_string())
    } else {
        None
    };

    Ok(ParsedEvent {
        title,
        start,
        duration_minutes,
        location,
    })
}

/// Parse a date/time string using chrono-english for natural language support
fn parse_datetime(input: &str) -> Result<DateTime<Utc>, ParseError> {
    // Get current time as the reference point
    let now = Local::now();

    // Normalize input for chrono-english:
    // - Remove "at" as chrono-english doesn't need it
    // - Remove "in" prefix for relative times (e.g., "in 2 hours" -> "2 hours")
    let normalized = input
        .replace(" at ", " ")
        .trim_start_matches("in ")
        .to_string();

    // Try chrono-english first for natural language parsing
    // Using US dialect for common date formats
    match parse_date_string(&normalized, now, Dialect::Us) {
        Ok(parsed) => Ok(parsed.with_timezone(&Utc)),
        Err(_) => {
            // Try standard ISO format as fallback
            if let Ok(dt) =
                DateTime::parse_from_str(&format!("{} +0000", input), "%Y-%m-%d %H:%M %z")
            {
                return Ok(dt.with_timezone(&Utc));
            }

            // Try without timezone
            if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M") {
                return Ok(naive.and_utc());
            }

            Err(ParseError::InvalidDateTime(input.to_string()))
        }
    }
}

/// Format examples for user help message
pub fn format_example() -> &'static str {
    r#"<b>Example:</b>
Team Meeting
tomorrow at 2pm
60
Conference Room A

<b>Format:</b>
Line 1: Event title
Line 2: Date/time (e.g., "tomorrow 2pm", "next Monday 10:00", "2026-01-25 14:00")
Line 3: Duration in minutes (optional, default: 60)
Line 4: Location (optional)"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, Timelike};

    #[test]
    fn test_parse_minimal_event() {
        let input = "Team Meeting\ntomorrow 2pm";
        let result = parse_event_message(input);
        assert!(result.is_ok());
        let event = result.expect("should parse");
        assert_eq!(event.title, "Team Meeting");
        assert_eq!(event.duration_minutes, 60); // default
        assert!(event.location.is_none());
    }

    #[test]
    fn test_parse_full_event() {
        let input = "Sprint Planning\n2026-01-25 10:00\n90\nConference Room B";
        let result = parse_event_message(input);
        assert!(result.is_ok());
        let event = result.expect("should parse");
        assert_eq!(event.title, "Sprint Planning");
        assert_eq!(event.duration_minutes, 90);
        assert_eq!(event.location, Some("Conference Room B".to_string()));
    }

    #[test]
    fn test_parse_with_iso_datetime() {
        let input = "Test Event\n2026-01-20 14:30\n30";
        let result = parse_event_message(input);
        assert!(result.is_ok());
        let event = result.expect("should parse");
        assert_eq!(event.start.with_timezone(&Local).hour(), 14);
        assert_eq!(event.start.with_timezone(&Local).minute(), 30);
    }

    #[test]
    fn test_missing_title() {
        let input = "\ntomorrow 2pm";
        let result = parse_event_message(input);
        assert!(matches!(result, Err(ParseError::MissingTitle)));
    }

    #[test]
    fn test_missing_datetime() {
        // Use two lines where second is empty (after trimming whitespace)
        let input = "Event Title\n   ";
        let result = parse_event_message(input);
        assert!(matches!(result, Err(ParseError::MissingDateTime)));
    }

    #[test]
    fn test_too_few_lines() {
        let input = "Just a title";
        let result = parse_event_message(input);
        assert!(matches!(result, Err(ParseError::TooFewLines)));
    }

    #[test]
    fn test_invalid_duration() {
        let input = "Event\ntomorrow 2pm\nnot_a_number";
        let result = parse_event_message(input);
        assert!(matches!(result, Err(ParseError::InvalidDuration)));
    }

    #[test]
    fn test_zero_duration() {
        let input = "Event\ntomorrow 2pm\n0";
        let result = parse_event_message(input);
        assert!(matches!(result, Err(ParseError::InvalidDuration)));
    }

    #[test]
    fn test_end_time_calculation() {
        let input = "Event\n2026-01-20 14:00\n90";
        let event = parse_event_message(input).expect("should parse");
        let end = event.end_time();
        assert_eq!(end.with_timezone(&Local).hour(), 15);
        assert_eq!(end.with_timezone(&Local).minute(), 30);
    }

    #[test]
    fn test_natural_language_dates() {
        // Test various natural language formats
        let test_cases = [
            "Event\ntomorrow at 2pm",
            "Event\nnext Monday 10:00",
            "Event\nin 2 hours",
        ];

        for input in test_cases {
            let result = parse_event_message(input);
            assert!(
                result.is_ok(),
                "Failed to parse: {} - {:?}",
                input,
                result.err()
            );
        }
    }

    #[test]
    fn test_whitespace_handling() {
        let input = "  Team Meeting  \n  tomorrow 2pm  \n  60  \n  Room A  ";
        let result = parse_event_message(input);
        assert!(result.is_ok());
        let event = result.expect("should parse");
        assert_eq!(event.title, "Team Meeting");
        assert_eq!(event.location, Some("Room A".to_string()));
    }
}
