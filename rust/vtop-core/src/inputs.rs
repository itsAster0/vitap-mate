//! Validated request inputs. Parsing happens once at the edge (FFI call or
//! HTTP request) so the fetchers only ever see well-formed values.

use crate::error::{VtopError, VtopResult};

const MAX_TEXT_LEN: usize = 256;

fn required_text(value: &str, label: &str) -> VtopResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(VtopError::ConfigurationError(format!(
            "{label} must not be empty"
        )));
    }
    if trimmed.len() > MAX_TEXT_LEN {
        return Err(VtopError::ConfigurationError(format!(
            "{label} is too long"
        )));
    }
    Ok(trimmed.to_owned())
}

macro_rules! required_text_type {
    ($(#[$meta:meta])* $name:ident, $label:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: &str) -> VtopResult<Self> {
                required_text(value, $label).map(Self)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

required_text_type!(
    /// A VTOP login name. Upper-cased, as VTOP expects.
    Username,
    "username"
);
required_text_type!(SemesterId, "semester_id");
required_text_type!(CourseId, "course_id");
required_text_type!(CourseType, "course_type");

impl Username {
    pub fn to_uppercase(&self) -> Self {
        Self(self.0.to_uppercase())
    }
}

/// A VTOP password. Kept verbatim (no trimming) and never printed.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    pub fn parse(value: &str) -> VtopResult<Self> {
        if value.trim().is_empty() {
            return Err(VtopError::ConfigurationError(
                "password must not be empty".to_string(),
            ));
        }
        if value.len() > MAX_TEXT_LEN {
            return Err(VtopError::ConfigurationError(
                "password is too long".to_string(),
            ));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Password(<redacted>)")
    }
}

/// The 6-digit security OTP VTOP emails on a new-device login.
pub struct OtpCode(String);

impl OtpCode {
    pub fn parse(raw: &str) -> VtopResult<Self> {
        let trimmed = raw.trim();
        if trimmed.len() == 6 && trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
            Ok(Self(trimmed.to_owned()))
        } else {
            Err(VtopError::AuthenticationFailed(
                "OTP must contain exactly 6 digits.".to_string(),
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for OtpCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OtpCode(<redacted>)")
    }
}

/// A calendar date in the `DD/MM/YYYY` form the biometric endpoint takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiometricDate(String);

impl BiometricDate {
    pub fn parse(raw: &str) -> VtopResult<Self> {
        let mut parts = raw.split('/');
        let (Some(day), Some(month), Some(year), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(Self::invalid());
        };
        if day.len() != 2 || month.len() != 2 || year.len() != 4 {
            return Err(Self::invalid());
        }
        let day = day.parse::<u32>().map_err(|_| Self::invalid())?;
        let month = month.parse::<u32>().map_err(|_| Self::invalid())?;
        let year = year.parse::<u32>().map_err(|_| Self::invalid())?;
        let leap_year =
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap_year => 29,
            2 => 28,
            _ => return Err(Self::invalid()),
        };
        if day == 0 || day > days_in_month {
            return Err(Self::invalid());
        }
        Ok(Self(raw.to_owned()))
    }

    fn invalid() -> VtopError {
        VtopError::ConfigurationError(
            "date must be a real calendar date in DD/MM/YYYY format".to_string(),
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// An outing date in the `DD-Mon-YYYY` form the outing forms post
/// (jQuery UI's `dd-M-yy`), e.g. `11-Oct-2026`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutingDate(String);

impl OutingDate {
    pub fn parse(raw: &str) -> VtopResult<Self> {
        let mut parts = raw.split('-');
        let (Some(day), Some(month), Some(year), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(Self::invalid());
        };
        let month = MONTHS
            .iter()
            .position(|name| *name == month)
            .ok_or_else(Self::invalid)?;
        // Reuse the calendar check rather than repeat it.
        BiometricDate::parse(&format!("{day}/{:02}/{year}", month + 1))
            .map_err(|_| Self::invalid())?;
        Ok(Self(raw.to_owned()))
    }

    fn invalid() -> VtopError {
        VtopError::ConfigurationError("outing date must look like 11-Oct-2026".to_string())
    }

    /// Days since 1970-01-01 (Howard Hinnant's days_from_civil).
    pub fn day_number(&self) -> i64 {
        let mut parts = self.0.split('-');
        let day: i64 = parts.next().and_then(|d| d.parse().ok()).unwrap_or(1);
        let month = parts
            .next()
            .and_then(|m| MONTHS.iter().position(|name| *name == m))
            .map_or(1, |index| index as i64 + 1);
        let year: i64 = parts.next().and_then(|y| y.parse().ok()).unwrap_or(1970);
        let year = if month <= 2 { year - 1 } else { year };
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A time of day as the general outing form posts it: two-digit hour and
/// minute fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutingTime {
    pub hour: u8,
    pub minute: u8,
}

impl OutingTime {
    pub fn new(hour: u8, minute: u8) -> VtopResult<Self> {
        if hour > 23 || minute > 59 {
            return Err(VtopError::ConfigurationError(
                "outing time must be a real time of day".to_string(),
            ));
        }
        Ok(Self { hour, minute })
    }

    pub fn hour_field(&self) -> String {
        format!("{:02}", self.hour)
    }

    pub fn minute_field(&self) -> String {
        format!("{:02}", self.minute)
    }
}

/// A 10-digit mobile number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactNumber(String);

impl ContactNumber {
    pub fn parse(raw: &str) -> VtopResult<Self> {
        let trimmed = raw.trim();
        if trimmed.len() != 10 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return Err(VtopError::ConfigurationError(
                "contact number must be 10 digits".to_string(),
            ));
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The id of an accepted outing's pass (`L…` or `W…`), which ends up in a
/// URL path, so only letters and digits are allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutingPassId(String);

impl OutingPassId {
    pub fn parse(raw: &str) -> VtopResult<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty()
            || trimmed.len() > 32
            || !trimmed.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Err(VtopError::ConfigurationError(
                "outing pass id is not valid".to_string(),
            ));
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A course page download path, as found in its `vtopDownload('…')` links,
/// e.g. `downloadPdf/AP2026272/AP2026272000046/19/14-07-2026`. Only the
/// endpoints the course page uses are allowed, with plain path characters,
/// since the path is put straight into the URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseFilePath(String);

impl CourseFilePath {
    const PREFIXES: [&'static str; 3] = [
        "downloadPdf/",
        "courseSyllabusDownload/",
        "academics/common/allCourseMeterialDownload/",
    ];

    pub fn parse(raw: &str) -> VtopResult<Self> {
        let path = raw.trim().trim_start_matches('/');
        let allowed = Self::PREFIXES.iter().any(|prefix| path.starts_with(prefix))
            && path.len() <= 200
            && !path.contains("..")
            && path
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_'));
        if !allowed {
            return Err(VtopError::ConfigurationError(
                "not a course page download".to_string(),
            ));
        }
        Ok(Self(path.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The student registration number VTOP embeds in every signed-in page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationNumber(String);

impl RegistrationNumber {
    pub fn parse(raw: &str) -> VtopResult<Self> {
        let value = raw.trim();
        if value.is_empty() {
            Err(VtopError::RegistrationParsingError)
        } else {
            Ok(Self(value.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RegistrationNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otp_code_requires_exactly_six_digits() {
        assert!(OtpCode::parse("123456").is_ok());
        assert!(OtpCode::parse(" 123456 ").is_ok());
        assert!(OtpCode::parse("12345").is_err());
        assert!(OtpCode::parse("12345x").is_err());
    }

    #[test]
    fn biometric_date_rejects_impossible_calendar_dates() {
        assert!(BiometricDate::parse("29/02/2024").is_ok());
        assert!(BiometricDate::parse("29/02/2023").is_err());
        assert!(BiometricDate::parse("31/04/2024").is_err());
        assert!(BiometricDate::parse("01/01/2024/1").is_err());
        assert!(BiometricDate::parse("1/1/2024").is_err());
    }

    #[test]
    fn registration_number_cannot_be_empty() {
        assert!(RegistrationNumber::parse("22BCE0001").is_ok());
        assert!(RegistrationNumber::parse("   ").is_err());
    }

    #[test]
    fn required_text_is_trimmed_and_bounded() {
        assert_eq!(SemesterId::parse(" AP2026 ").unwrap().as_str(), "AP2026");
        assert!(SemesterId::parse("  ").is_err());
        assert!(CourseId::parse(&"x".repeat(257)).is_err());
    }

    #[test]
    fn secrets_are_redacted_in_debug_output() {
        let password = Password::parse("hunter2").unwrap();
        assert!(!format!("{password:?}").contains("hunter2"));
        let otp = OtpCode::parse("123456").unwrap();
        assert!(!format!("{otp:?}").contains("123456"));
    }

    #[test]
    fn outing_inputs() {
        assert!(OutingDate::parse("11-Oct-2026").is_ok());
        assert!(OutingDate::parse("29-Feb-2026").is_err());
        assert!(OutingDate::parse("11-oct-2026").is_err());
        assert!(OutingDate::parse("2026-10-11").is_err());
        assert!(OutingTime::new(22, 0).is_ok());
        assert!(OutingTime::new(24, 0).is_err());
        assert_eq!(OutingTime::new(6, 5).unwrap().minute_field(), "05");
        assert!(ContactNumber::parse(" 9876543210 ").is_ok());
        assert!(ContactNumber::parse("98765abcde").is_err());
        assert!(OutingPassId::parse("W24859931194").is_ok());
        assert!(OutingPassId::parse("../x").is_err());
    }

    #[test]
    fn course_file_paths_are_limited_to_the_course_page() {
        assert!(
            CourseFilePath::parse("downloadPdf/AP2026272/AP2026272000046/19/14-07-2026").is_ok()
        );
        assert!(CourseFilePath::parse("courseSyllabusDownload/AM_CSE4007_00100/ETH").is_ok());
        assert!(CourseFilePath::parse(
            "academics/common/allCourseMeterialDownload/1/1/AP2026272/AP2026272000046"
        )
        .is_ok());
        assert!(CourseFilePath::parse("downloadPdf/../logout").is_err());
        assert!(CourseFilePath::parse("hostel/saveOutingForm").is_err());
        assert!(CourseFilePath::parse("downloadPdf/a?x=1").is_err());
    }

    #[test]
    fn outing_date_day_numbers() {
        assert_eq!(OutingDate::parse("01-Jan-1970").unwrap().day_number(), 0);
        assert_eq!(
            OutingDate::parse("11-Oct-2026").unwrap().day_number(),
            20_737
        );
        assert_eq!(
            OutingDate::parse("01-Mar-2024").unwrap().day_number(),
            19_783
        );
    }
}
