//! Calendar days as numbers: days since 1970-01-01, in India's time zone
//! where VTOP's dates are.

use crate::now_unix;

/// Weekday names, Sunday first, as [`weekday`] numbers them.
pub const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Days since 1970-01-01 of a valid date (Howard Hinnant's
/// days_from_civil).
pub fn day_number(day: i64, month: i64, year: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// (year, month, day) of a day number (civil_from_days).
pub fn civil_date(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Today's day number in India (UTC+5:30).
pub fn today_in_india() -> i64 {
    (now_unix() as i64 + 19_800).div_euclid(86_400)
}

/// Minutes since 1970-01-01 00:00 in India, now.
pub fn minutes_in_india() -> i64 {
    (now_unix() as i64 + 19_800).div_euclid(60)
}

/// 0 = Sunday, for a day number (1970-01-01 was a Thursday).
pub fn weekday(days: i64) -> u8 {
    (days + 4).rem_euclid(7) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_numbers() {
        assert_eq!(day_number(1, 1, 1970), 0);
        assert_eq!(day_number(11, 10, 2026), 20_737);
        assert_eq!(day_number(1, 3, 2024), 19_783);
        assert_eq!(day_number(29, 2, 2024), 19_782);
        assert_eq!(day_number(31, 12, 1969), -1);
    }

    #[test]
    fn civil_dates_round_trip() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(19_782), (2024, 2, 29));
        assert_eq!(civil_date(-1), (1969, 12, 31));
        for days in -800_000..800_000 {
            let (year, month, day) = civil_date(days);
            assert_eq!(day_number(day, month, year), days);
        }
    }

    #[test]
    fn weekdays() {
        assert_eq!(WEEKDAYS[usize::from(weekday(0))], "Thursday");
        assert_eq!(WEEKDAYS[usize::from(weekday(20_737))], "Sunday");
        assert_eq!(WEEKDAYS[usize::from(weekday(-1))], "Wednesday");
    }

    #[test]
    fn today_is_after_2026() {
        assert!(today_in_india() >= day_number(1, 1, 2026));
    }
}
