//! Each VTOP page a screen shows: its feature's `#[uniffi::export] impl Core` block
//! (`cached_x`, `fetch_x`) and ready-to-draw records, plus the change inbox and Gmail's OTP.

pub(crate) mod attendance;
pub(crate) mod biometric;
pub(crate) mod biometric_job;
pub(crate) mod calendar;
pub(crate) mod changes;
pub(crate) mod course_page;
pub(crate) mod exams;
pub(crate) mod gmail;
pub(crate) mod grades;
pub(crate) mod marks;
pub(crate) mod outing;
pub(crate) mod profile;
pub(crate) mod records;
pub(crate) mod timetable;
pub(crate) mod wrapped;
