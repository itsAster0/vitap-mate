//! Packs a parsed page for a phone that holds the same page dictionary
//! (`vtop_shared::page_dictionary`): Brotli at its best quality, with that
//! page's dictionary. The tests keep the dictionary's page shapes in step
//! with vtop-core's parser fixtures.

use std::io::{Error, ErrorKind};

use brotli::enc::{interface, BrotliEncoderParams, StandardAlloc};
use brotli::{InputReferenceMut, IoReaderWrapper, IoWriterWrapper};
use serde_json::Value;
use vtop_shared::page_dictionary;

/// Page `name` packed with its dictionary; None if it cannot be, and the
/// page goes as JSON instead.
pub fn pack(name: &str, page: &Value) -> Option<Vec<u8>> {
    let json = serde_json::to_vec(page).ok()?;
    let params = BrotliEncoderParams {
        quality: 11,
        ..Default::default()
    };
    let mut packed = Vec::new();
    let mut no_log = |_: &mut interface::PredictionModeContextMap<InputReferenceMut>,
                      _: &mut [interface::StaticCommand],
                      _: interface::InputPair,
                      _: &mut StandardAlloc| ();
    brotli::BrotliCompressCustomIoCustomDict(
        &mut IoReaderWrapper(&mut json.as_slice()),
        &mut IoWriterWrapper(&mut packed),
        &mut [0; 4096],
        &mut [0; 4096],
        &params,
        StandardAlloc::default(),
        &mut no_log,
        &page_dictionary::for_page(name),
        Error::new(ErrorKind::UnexpectedEof, "Unexpected EOF"),
    )
    .ok()?;
    Some(packed)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use serde_json::json;
    use vtop_core::types::{
        CoursePageClasses, CoursePageCourses, CoursePageDetail, GeneralOutingData,
        WeekendOutingData,
    };
    use vtop_shared::page_dictionary::{unpack, Source, SOURCE};

    use super::*;
    use crate::pages;

    /// vtop-core fixture → the page it is the output of.
    const FIXTURES: &[(&str, &str)] = &[
        ("attendance", "attendance"),
        ("biometric", "biometric"),
        ("calendar_month", "academic_calendar"),
        ("exam_schedule", "exam_schedule"),
        ("full_attendance", "full_attendance"),
        ("grade_details", "grade_details"),
        ("grade_history", "grade_history"),
        ("grade_view", "grades"),
        ("marks", "marks"),
        ("semesters", "semesters"),
        ("timetable", "timetable"),
    ];

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../vtop-core/tests/fixtures")
            .join(format!("{name}.expected.json"));
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
    }

    /// A page as `academic_calendar` answers it: the fixture is one month's
    /// entries.
    fn page(fixture_name: &str, page: &str) -> Value {
        let value = fixture(fixture_name);
        match page {
            "academic_calendar" => json!({ "entries": value, "semester_id": "", "update_time": 0 }),
            _ => value,
        }
    }

    /// Made-up pages for those without a parser fixture.
    const SAMPLES: &str = include_str!("page_samples.json");

    fn samples() -> BTreeMap<String, Value> {
        serde_json::from_str(SAMPLES).unwrap()
    }

    /// Every page with a fixture or a sample, by page name.
    fn all_pages() -> BTreeMap<String, Value> {
        let mut all = samples();
        for (fixture, name) in FIXTURES {
            all.insert(name.to_string(), page(fixture, name));
        }
        all
    }

    /// `sample` read as vtop-core's `T` and written back.
    fn as_type<T: DeserializeOwned + Serialize>(sample: &Value) -> Value {
        let typed: T = serde_json::from_value(sample.clone()).unwrap();
        serde_json::to_value(typed).unwrap()
    }

    /// Strings emptied unless they are vocabulary words, numbers zeroed, at
    /// most two items per list.
    fn shape(value: &Value, vocabulary: &[String]) -> Value {
        match value {
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), shape(v, vocabulary)))
                    .collect(),
            ),
            Value::Array(items) => {
                Value::Array(items.iter().take(2).map(|v| shape(v, vocabulary)).collect())
            }
            Value::String(text) if vocabulary.contains(text) => value.clone(),
            Value::String(_) => Value::String(String::new()),
            Value::Number(_) => Value::from(0),
            other => other.clone(),
        }
    }

    /// A field added to or dropped from vtop-core's types fails here, so
    /// the samples (and the shapes made from them) keep up.
    #[test]
    fn the_samples_are_vtop_cores_types() {
        for (name, sample) in samples() {
            let typed = match name.as_str() {
                "course_page_classes" => as_type::<CoursePageClasses>(&sample),
                "course_page_courses" => as_type::<CoursePageCourses>(&sample),
                "course_page_detail" => as_type::<CoursePageDetail>(&sample),
                "general_outing" => as_type::<GeneralOutingData>(&sample),
                "weekend_outing" => as_type::<WeekendOutingData>(&sample),
                other => panic!("no type for the sample {other}"),
            };
            assert_eq!(typed, sample, "{name} no longer matches its type");
        }
    }

    /// Regenerate after a parser change with
    /// `UPDATE_SNAPSHOTS=1 cargo test -p vtop-bridge page_pack`.
    #[test]
    fn the_dictionary_has_every_fixture_page_shape() {
        let source: Source = serde_json::from_str(SOURCE).unwrap();
        let shapes: BTreeMap<String, Value> = all_pages()
            .iter()
            .map(|(name, page)| (name.clone(), shape(page, &source.vocabulary)))
            .collect();
        if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
            let file = json!({ "vocabulary": source.vocabulary, "shapes": shapes });
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../vtop-shared/src/page_dictionary.json");
            std::fs::write(path, serde_json::to_string_pretty(&file).unwrap() + "\n").unwrap();
            return;
        }
        assert_eq!(
            source.shapes, shapes,
            "page_dictionary.json is out of date; rerun with UPDATE_SNAPSHOTS=1"
        );
        for name in shapes.keys() {
            assert!(pages::find(name).is_some(), "{name} is not a page");
        }
    }

    #[test]
    fn every_page_unpacks_as_it_was_and_smaller_than_plain_brotli() {
        let (mut packed_total, mut plain_total) = (0, 0);
        for (fixture, name) in FIXTURES {
            let page = page(fixture, name);
            let packed = pack(name, &page).unwrap();
            let json = unpack(name, &packed).unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&json).unwrap(),
                page,
                "{name}"
            );
            let mut plain = Vec::new();
            brotli::BrotliCompress(
                &mut json.as_slice(),
                &mut plain,
                &BrotliEncoderParams {
                    quality: 11,
                    ..Default::default()
                },
            )
            .unwrap();
            packed_total += packed.len();
            plain_total += plain.len();
        }
        println!("packed {packed_total} bytes, plain Brotli {plain_total} bytes");
        assert!(
            packed_total * 100 < plain_total * 80,
            "{packed_total} vs {plain_total}"
        );
    }

    #[test]
    fn a_page_without_a_shape_still_round_trips() {
        let page = json!({ "pdf": "JVBERi0xLjQK" });
        let packed = pack("outing_pass", &page).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&unpack("outing_pass", &packed).unwrap()).unwrap(),
            page
        );
    }

    #[test]
    fn another_pages_dictionary_does_not_unpack_it() {
        let page = page("marks", "marks");
        let packed = pack("marks", &page).unwrap();
        let wrong = unpack("timetable", &packed)
            .ok()
            .and_then(|json| serde_json::from_slice::<Value>(&json).ok());
        assert_ne!(wrong, Some(page));
    }
}
