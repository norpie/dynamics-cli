use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc, LocalResult, Duration, TimeZone};
use chrono_tz::Europe::Brussels;
use log::debug;

/// Combines a Brussels local date and time into a UTC datetime for storage
///
/// # Arguments
/// * `date` - The date in Brussels local time
/// * `time` - The time in Brussels local time (defaults to 12:00 if None)
///
/// # Returns
/// * UTC datetime ready for storage in Dynamics 365
///
/// # Handles
/// * DST transitions automatically
/// * Spring forward gaps (02:30 doesn't exist)
/// * Fall back ambiguity (02:30 exists twice - uses earlier occurrence)
pub fn combine_brussels_datetime(date: NaiveDate, time: Option<NaiveTime>) -> Result<DateTime<Utc>> {
    // Use 12:00 Brussels local time as default if no time provided
    let local_time = time.unwrap_or_else(|| NaiveTime::from_hms_opt(12, 0, 0).unwrap());

    let brussels_naive = date.and_time(local_time);
    debug!("Converting Brussels local time: {} to UTC", brussels_naive);

    // Convert Brussels local time to UTC, handling DST automatically
    match Brussels.from_local_datetime(&brussels_naive) {
        LocalResult::Single(brussels_dt) => {
            let utc_dt = brussels_dt.with_timezone(&Utc);
            debug!("Brussels {} -> UTC {}", brussels_dt, utc_dt);
            Ok(utc_dt)
        },
        LocalResult::Ambiguous(earlier, _later) => {
            // Fall back transition: time exists twice
            // Use the earlier occurrence (before the clocks fall back)
            let utc_dt = earlier.with_timezone(&Utc);
            debug!("Ambiguous time resolved: Brussels {} (earlier) -> UTC {}", earlier, utc_dt);
            Ok(utc_dt)
        },
        LocalResult::None => {
            // Spring forward transition: time doesn't exist
            // This happens during 02:00-03:00 gap in spring
            Err(anyhow!(
                "Invalid Brussels local time: {} (during DST spring forward transition)",
                brussels_naive
            ))
        }
    }
}

/// Parse time string in HH:MM format to NaiveTime
///
/// # Arguments
/// * `time_str` - Time string like "10:00", "14:30", etc.
///
/// # Returns
/// * Parsed time or error if invalid format
pub fn parse_time_string(time_str: &str) -> Result<NaiveTime> {
    let time_str = time_str.trim();

    if !time_str.contains(':') {
        return Err(anyhow!("Invalid time format: '{}' (expected HH:MM)", time_str));
    }

    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid time format: '{}' (expected HH:MM)", time_str));
    }

    let hour: u32 = parts[0].parse()
        .map_err(|_| anyhow!("Invalid hour: '{}'", parts[0]))?;
    let minute: u32 = parts[1].parse()
        .map_err(|_| anyhow!("Invalid minute: '{}'", parts[1]))?;

    NaiveTime::from_hms_opt(hour, minute, 0)
        .ok_or_else(|| anyhow!("Invalid time: {}:{} (hour must be 0-23, minute 0-59)", hour, minute))
}

/// Convert Excel serial date number to NaiveDate
///
/// Excel stores dates as serial numbers since 1900-01-01
/// This handles the Excel leap year bug (treats 1900 as leap year)
pub fn excel_serial_to_date(serial: f64) -> Result<NaiveDate> {
    // Excel epoch: 1900-01-01 (but Excel thinks 1900 is a leap year)
    // Serial 1 = 1900-01-01, Serial 60 = 1900-02-29 (doesn't exist), Serial 61 = 1900-03-01

    if serial < 1.0 {
        return Err(anyhow!("Invalid Excel date serial: {} (must be >= 1)", serial));
    }

    let excel_epoch = NaiveDate::from_ymd_opt(1900, 1, 1)
        .ok_or_else(|| anyhow!("Failed to create Excel epoch date"))?;

    // Adjust for Excel's leap year bug: if serial >= 60, subtract 1 day
    let adjusted_days = if serial >= 60.0 {
        (serial - 2.0) as i64  // -1 for 0-based, -1 for leap year bug
    } else {
        (serial - 1.0) as i64  // -1 for 0-based
    };

    excel_epoch.checked_add_signed(Duration::days(adjusted_days))
        .ok_or_else(|| anyhow!("Invalid Excel date serial: {} (overflow)", serial))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_parse_time_string() {
        assert_eq!(parse_time_string("10:30").unwrap(), NaiveTime::from_hms_opt(10, 30, 0).unwrap());
        assert_eq!(parse_time_string("00:00").unwrap(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        assert_eq!(parse_time_string("23:59").unwrap(), NaiveTime::from_hms_opt(23, 59, 0).unwrap());

        assert!(parse_time_string("25:00").is_err()); // Invalid hour
        assert!(parse_time_string("10:60").is_err()); // Invalid minute
        assert!(parse_time_string("10").is_err());    // Missing colon
        assert!(parse_time_string("abc").is_err());   // Invalid format
    }

    #[test]
    fn test_combine_brussels_datetime_summer() {
        // Test summer time (CEST = UTC+2)
        let date = NaiveDate::from_ymd_opt(2025, 7, 15).unwrap();
        let time = NaiveTime::from_hms_opt(14, 30, 0);

        let utc_dt = combine_brussels_datetime(date, time).unwrap();

        // 14:30 CEST = 12:30 UTC
        assert_eq!(utc_dt.hour(), 12);
        assert_eq!(utc_dt.minute(), 30);
    }

    #[test]
    fn test_combine_brussels_datetime_winter() {
        // Test winter time (CET = UTC+1)
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let time = NaiveTime::from_hms_opt(14, 30, 0);

        let utc_dt = combine_brussels_datetime(date, time).unwrap();

        // 14:30 CET = 13:30 UTC
        assert_eq!(utc_dt.hour(), 13);
        assert_eq!(utc_dt.minute(), 30);
    }

    #[test]
    fn test_combine_brussels_datetime_default_time() {
        // Test default 12:00 time
        let date = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();

        let utc_dt = combine_brussels_datetime(date, None).unwrap();

        // 12:00 CEST = 10:00 UTC (summer time)
        assert_eq!(utc_dt.hour(), 10);
        assert_eq!(utc_dt.minute(), 0);
    }

    #[test]
    fn test_excel_serial_to_date() {
        // Test known Excel serial dates
        assert_eq!(excel_serial_to_date(44927.0).unwrap(), NaiveDate::from_ymd_opt(2023, 1, 1).unwrap());
        assert_eq!(excel_serial_to_date(45292.0).unwrap(), NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());

        // Test Excel leap year bug boundary
        assert_eq!(excel_serial_to_date(59.0).unwrap(), NaiveDate::from_ymd_opt(1900, 2, 28).unwrap());
        assert_eq!(excel_serial_to_date(61.0).unwrap(), NaiveDate::from_ymd_opt(1900, 3, 1).unwrap());

        // Test invalid serials
        assert!(excel_serial_to_date(0.0).is_err());
        assert!(excel_serial_to_date(-1.0).is_err());
    }
}