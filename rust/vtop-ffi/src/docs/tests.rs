use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::*;
use crate::Core;
use crate::app_files::{StoredSavedFile, decode_doc_index};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-docs-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn doc(id: &str, name: &str, added: i64, opened: Option<i64>, kind: DocKind, size: i64) -> Doc {
    Doc {
        id: id.into(),
        name: name.into(),
        file: format!("{id}.pdf"),
        uri: Some(format!("content://test/{id}")),
        kind,
        size,
        source: "import".into(),
        added_at: added,
        last_opened_at: opened,
        as_mess_menu: false,
        page: 0,
        zoom: 1.0,
    }
}

fn sample_docs() -> Vec<Doc> {
    vec![
        doc("a", "Mess menu", 3, Some(10), DocKind::Pdf, 1),
        doc("b", "outing pass", 1, None, DocKind::Pdf, 1),
        doc("c", "Fee receipt", 2, Some(20), DocKind::Pdf, 1),
        doc("d", "Calendar", 4, None, DocKind::Pdf, 1),
    ]
}

fn ids(docs: Vec<Doc>) -> Vec<String> {
    docs.into_iter().map(|d| d.id).collect()
}

#[test]
fn recently_opened_puts_never_opened_last_by_name() {
    assert_eq!(
        ids(query_docs(
            sample_docs(),
            DocSort::RecentlyOpened,
            String::new(),
            None
        )),
        vec!["c", "a", "d", "b"]
    );
}

#[test]
fn other_sorts() {
    assert_eq!(
        ids(query_docs(
            sample_docs(),
            DocSort::RecentlyAdded,
            String::new(),
            None
        )),
        vec!["d", "a", "c", "b"]
    );
    assert_eq!(
        ids(query_docs(
            sample_docs(),
            DocSort::NameAscending,
            String::new(),
            None
        )),
        vec!["d", "c", "a", "b"]
    );
    assert_eq!(
        ids(query_docs(
            sample_docs(),
            DocSort::NameDescending,
            String::new(),
            None
        )),
        vec!["b", "a", "c", "d"]
    );
}

#[test]
fn largest_and_type_sorts() {
    let mixed = vec![
        doc("p", "b pass", 1, None, DocKind::Pdf, 50),
        doc("i", "photo", 1, None, DocKind::Image, 900),
        doc("s", "menu", 1, None, DocKind::Spreadsheet, 50),
        doc("q", "a notes", 1, None, DocKind::Pdf, 10),
    ];
    assert_eq!(
        ids(query_docs(
            mixed.clone(),
            DocSort::Largest,
            String::new(),
            None
        )),
        vec!["i", "p", "s", "q"]
    );
    assert_eq!(
        ids(query_docs(
            mixed.clone(),
            DocSort::Type,
            String::new(),
            None
        )),
        vec!["q", "p", "i", "s"]
    );
    assert_eq!(
        ids(query_docs(
            mixed,
            DocSort::NameAscending,
            String::new(),
            Some(DocKind::Pdf)
        )),
        vec!["q", "p"]
    );
}

#[test]
fn search_ignores_case_and_spaces() {
    assert_eq!(
        ids(query_docs(
            sample_docs(),
            DocSort::NameDescending,
            "  MENU ".into(),
            None
        )),
        vec!["a"]
    );
    assert_eq!(
        ids(query_docs(
            sample_docs(),
            DocSort::RecentlyOpened,
            "e".into(),
            None
        )),
        vec!["c", "a", "d"]
    );
    assert_eq!(
        ids(query_docs(
            sample_docs(),
            DocSort::NameAscending,
            "PASS".into(),
            None
        )),
        vec!["b"]
    );
    assert!(query_docs(sample_docs(), DocSort::NameAscending, "zzz".into(), None).is_empty());
}

#[test]
fn ties_fall_back_to_name_then_id() {
    let same = vec![
        doc("2", "x", 1, None, DocKind::Pdf, 1),
        doc("1", "x", 1, None, DocKind::Pdf, 1),
        doc("3", "a", 1, None, DocKind::Pdf, 1),
    ];
    assert_eq!(
        ids(query_docs(
            same,
            DocSort::RecentlyAdded,
            String::new(),
            None
        )),
        vec!["3", "1", "2"]
    );
}

#[test]
fn the_mess_menu_is_the_newest_marked_one() {
    let mut menus = sample_docs();
    let mut m1 = doc("m1", "Menu Oct", 5, None, DocKind::Spreadsheet, 1);
    m1.as_mess_menu = true;
    let mut m2 = doc("m2", "Menu Nov", 9, None, DocKind::Spreadsheet, 1);
    m2.as_mess_menu = true;
    menus.push(m1);
    menus.push(m2);
    assert_eq!(newest_mess_menu(menus).map(|d| d.id), Some("m2".into()));
    assert_eq!(newest_mess_menu(sample_docs()), None);
}

#[test]
fn kind_from_extension() {
    assert_eq!(doc_kind_of("a.PDF".into()), DocKind::Pdf);
    assert_eq!(doc_kind_of("x.jpeg".into()), DocKind::Image);
    assert_eq!(doc_kind_of("menu.xlsx".into()), DocKind::Spreadsheet);
    assert_eq!(doc_kind_of("notes.md".into()), DocKind::Text);
    assert_eq!(doc_kind_of("archive.zip".into()), DocKind::File);
    assert_eq!(doc_kind_of("noext".into()), DocKind::File);
}

#[test]
fn existing_index_json_reads_unchanged() {
    let dir = scratch("existing-index");
    let docs_dir = dir.join("docs");
    fs::create_dir_all(&docs_dir).unwrap();
    let fixture = r#"{"sort":"RecentlyAdded","docs":[{"id":"doc-1","name":"Mess menu","file":"Mess menu.xlsx","uri":"content://media/1","kind":"Spreadsheet","size":2048,"source":"import","addedAt":1700000000000,"lastOpenedAt":null,"asMessMenu":true,"page":2,"zoom":1.5},{"id":"doc-2","name":"Syllabus","file":"Syllabus.pdf","uri":"content://media/2","kind":"","size":1024,"source":"coursepage","addedAt":1700000005000,"lastOpenedAt":1700000009000,"asMessMenu":false,"page":0,"zoom":1}]}"#;
    fs::write(docs_dir.join("index.json"), fixture).unwrap();

    let core = Core::for_test(&dir);
    assert_eq!(core.docs_sort("f".into()), DocSort::RecentlyAdded);
    let list = core.docs("f".into());
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "doc-1");
    assert_eq!(list[0].kind, DocKind::Spreadsheet);
    assert!(list[0].as_mess_menu);
    assert_eq!(list[0].page, 2);
    assert_eq!(list[0].zoom, 1.5);
    // Empty kind in index.json falls back to extension.
    assert_eq!(list[1].id, "doc-2");
    assert_eq!(list[1].kind, DocKind::Pdf);
    assert_eq!(list[1].last_opened_at, Some(1_700_000_009_000));
    assert_eq!(
        core.docs_newest_mess_menu("f".into()).map(|d| d.id),
        Some("doc-1".into())
    );
    assert_eq!(
        ids(core.docs_query("f".into(), DocSort::RecentlyAdded, String::new(), None)),
        vec!["doc-2", "doc-1"]
    );

    // Writing back keeps the exact StoredDocIndex format.
    core.docs_sort_set("f".into(), DocSort::NameAscending);
    let raw = fs::read_to_string(docs_dir.join("index.json")).unwrap();
    let stored = decode_doc_index(raw).unwrap();
    assert_eq!(stored.sort, "NameAscending");
    assert_eq!(stored.docs.len(), 2);
    assert_eq!(stored.docs[0].id, "doc-1");
    assert_eq!(stored.docs[1].kind, "Pdf");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rename_blank_keeps_old_name() {
    let dir = scratch("rename-blank");
    let core = Core::for_test(&dir);
    core.doc_add("f".into(), doc("a", "Original", 1, None, DocKind::Pdf, 10));

    let after_blank = core.doc_rename("f".into(), "a".into(), "   ".into());
    assert_eq!(after_blank[0].name, "Original");

    let after_trim = core.doc_rename("f".into(), "a".into(), "  Renamed  ".into());
    assert_eq!(after_trim[0].name, "Renamed");

    // Reopen verifies persistence.
    let reopened = Core::for_test(&dir);
    assert_eq!(reopened.docs("f".into())[0].name, "Renamed");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn remove_returns_doc_for_file_delete() {
    let dir = scratch("remove");
    let core = Core::for_test(&dir);
    let added = doc("a", "Pass", 1, None, DocKind::Pdf, 42);
    core.doc_add("f".into(), added.clone());

    let removed = core.doc_remove("f".into(), "a".into());
    assert_eq!(removed, Some(added));
    assert!(core.docs("f".into()).is_empty());
    assert_eq!(core.doc_remove("f".into(), "a".into()), None);

    let reopened = Core::for_test(&dir);
    assert!(reopened.docs("f".into()).is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn operations_reconcile_remap_and_downloads_persist() {
    let dir = scratch("ops");
    let core = Core::for_test(&dir);
    core.doc_add(
        "f".into(),
        doc("d1", "Menu", 10, None, DocKind::Spreadsheet, 100),
    );
    core.doc_set_mess_menu("f".into(), "d1".into(), true);
    core.doc_opened("f".into(), "d1".into(), 555);
    core.doc_save_view("f".into(), "d1".into(), 3, 2.25);

    let d = &core.docs("f".into())[0];
    assert!(d.as_mess_menu);
    assert_eq!(d.last_opened_at, Some(555));
    assert_eq!(d.page, 3);
    assert_eq!(d.zoom, 2.25);

    // Remap updates URI after folder move.
    let remapped = core.docs_remap(
        "f".into(),
        HashMap::from([("content://test/d1".into(), "content://moved/d1".into())]),
    );
    assert_eq!(remapped[0].uri.as_deref(), Some("content://moved/d1"));

    // Reconcile handles rename (matched by size + kind) and external addition.
    let rec = core.docs_reconcile(
        "f".into(),
        vec![
            FolderFile {
                uri: "content://moved/d1-renamed".into(),
                name: "Nov Menu.xlsx".into(),
                size: 100,
            },
            FolderFile {
                uri: "content://moved/extra".into(),
                name: "Notes.pdf".into(),
                size: 50,
            },
        ],
        999,
    );
    assert!(rec.changed);
    assert_eq!(rec.renamed, 1);
    assert_eq!(rec.added, 1);
    assert_eq!(rec.removed, 0);
    assert_eq!(rec.docs.len(), 2);
    assert_eq!(rec.docs[0].name, "Nov Menu");
    assert_eq!(rec.docs[1].source, "folder");

    // Downloads bookkeeping.
    assert!(core.downloads_needs_import("f".into()));
    core.downloads_import(
        "f".into(),
        vec![StoredSavedFile {
            uri: "content://tree/1".into(),
            name: "pass.pdf".into(),
            mime: "application/pdf".into(),
        }],
    )
    .unwrap();
    assert!(!core.downloads_needs_import("f".into()));
    core.downloads_saved(
        "f".into(),
        "menu.xlsx".into(),
        "content://tree/2".into(),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".into(),
    );
    core.downloads_moved(
        "f".into(),
        "content://tree/1".into(),
        "content://tree/1-new".into(),
    );
    let by_name = core.downloads_saved_by_name("f".into());
    assert_eq!(
        by_name.get("pass.pdf").map(String::as_str),
        Some("content://tree/1-new")
    );
    assert_eq!(
        by_name.get("menu.xlsx").map(String::as_str),
        Some("content://tree/2")
    );

    core.downloads_forget("f".into(), "content://tree/2".into());
    let reopened = Core::for_test(&dir);
    assert!(!reopened.downloads_needs_import("f".into()));
    assert_eq!(reopened.downloads_saved_list("f".into()).len(), 1);
    assert_eq!(
        reopened
            .downloads_saved_by_name("f".into())
            .get("pass.pdf")
            .map(String::as_str),
        Some("content://tree/1-new")
    );
    let _ = fs::remove_dir_all(&dir);
}
