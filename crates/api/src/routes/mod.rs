//! API route modules

/// Maximum number of hrefs allowed in a calendar-multiget report
pub const MAX_MULTIGET_HREFS: usize = 200;

pub mod caldav;
pub mod calendars;

mod caldav_xml;
pub mod devices;
pub mod events;
pub mod health;
mod ical;
pub mod me;
