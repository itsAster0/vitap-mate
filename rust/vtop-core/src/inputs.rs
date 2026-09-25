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
}
