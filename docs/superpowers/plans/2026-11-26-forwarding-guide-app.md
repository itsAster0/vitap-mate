# Forwarding guide, Part 1 (the app) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One list of forwarding steps in the app, driven by Rust. It names a wrong
Gmail account, shows a dismissible typed-OTP hint, and cleans up Gmail on Turn off. No
server changes.

**Architecture:** Rust (`rust/vtop-ffi/src/remote/forwarding.rs`) gains:
- `ConfirmFrom`, which says who the confirmation came from.
- `SetupStep`, the step the student is on.
- `ForwardingHint`, the typed-OTP hint.
- A side file, `forwarding_progress.json`, for state an older app must never see.

Kotlin (`features/login`) draws the steps, a "Show me" sheet built from
`guides/forwarding.json` blocks tagged with `"step"`, and a Turn off sheet. Kotlin
computes none of this; it only draws what Rust returns.

**Tech Stack:** Rust 2024 crate `vtop-ffi` with UniFFI; Kotlin, Compose Multiplatform
and Material 3 Expressive through the Kotlin Toolchain; jj.

**Spec:** `docs/superpowers/specs/2026-11-26-forwarding-guide-design.md` (Part 1, plus
"Compatibility").

## Global Constraints

- Nothing waits for, or is locked by, a VTOP OTP. `ForwardingUses` and
  `ForwardingReadiness` keep their meaning. Background sync stays allowed at
  `NeedsFilter | Ready`.
- `Stored` (the phone's `forwarding.json`, with `deny_unknown_fields`) is not changed.
  New local state goes only in `forwarding_progress.json`.
- `backend/guides/forwarding.json` keeps every existing block, its order and its
  wording. Only the optional `"step"` key is added.
- Theme colours and M3 Expressive only. Reuse `Callout`, `Step`, `heroCookie`,
  `Button` and `AppTheme.tones`.
- Every user action is a `flow()`, and its id goes first in every `Core` call. Never log
  codes, links or addresses.
- No `let _ =` on a fallible call without a reason comment. Use `#[expect]`, not
  `#[allow]`.
- No commits or pushes. The user commits. Use `jj` only to look.
- After any Rust change that Kotlin uses, run `mise run rust-android-dev` before
  building Kotlin.
- Keep the existing test tags (`forwarding.address`, `forwarding.copy`,
  `forwarding.status`, `forwarding.step<n>`, `forwarding.done`, `forwarding.check`,
  `forwarding.off`, `forwarding.off.confirm`, `forwarding.confirm_code`,
  `forwarding.confirm_link`). New ones are named in each task.

## Review Focus

1. **The requester comes in odd forms:** `"Name <a.22bce0001@vitapstudent.ac.in>"`,
   upper case, or padded with spaces. Expected: `CollegeInbox`. Covered in Task 1.
2. **The username isn't a regno,** or nobody is signed in. Expected: fall back to an
   exact `vit_email` match, then the profile, then `Unknown` or `NotSaid`. Never panic.
   Covered in Task 1.
3. **The address changes** (rejoined a different id) or forwarding is turned off and set
   up again. Expected: old progress and an old hint don't carry over. Covered in Task 2.
4. **An older app reads `forwarding.json` after the new app wrote it.** Expected: it still
   parses, and the side file is ignored. Covered in Task 2.
5. **A code is typed by hand while forwarding isn't Ready, or while Gmail is the OTP
   source.** Expected: no hint. A later typed OTP after a dismiss shows the hint again.
   Covered in Tasks 2 and 6.

---

### Task 1: Who confirmed (`ConfirmFrom`)

**Files:**
- Modify: `rust/vtop-ffi/src/remote/forwarding.rs`: `GmailConfirmation`
  (around lines 44–54), the `requester_known` sites (lines 117, 286, 966–982, 1043–1053)
- Test: `rust/vtop-ffi/src/remote/forwarding/tests.rs`: replace
  `knows_a_requester_on_the_profile` (line 509)

**Interfaces:**
- Produces:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)] pub enum ConfirmFrom { CollegeInbox, OwnOther, Unknown, NotSaid }`
  - `GmailConfirmation { confirm_by, requester, sent_at, from: ConfirmFrom }`. The
    `requester_known` field is removed.
  - A pure function:
    `fn confirm_from(requester: Option<&str>, regno: Option<&str>, profile: Option<&StudentProfile>) -> ConfirmFrom`

- [ ] **Step 1: Write the failing test `tells_who_confirmed`**

```rust
#[test]
fn tells_who_confirmed() {
    let profile: StudentProfile = serde_json::from_value(json!({
        "name": "A", "registration_number": "22BCE0001", "vit_email": "odd.name@vitapstudent.ac.in",
        "program": "", "school": "", "photo": "", "update_time": 0,
        "sections": [{ "title": "PERSONAL", "groups": [{ "title": "", "fields": [
            { "label": "Email", "value": "Me@Gmail.com, other@x.in" } ] }] }],
    })).unwrap();
    let p = Some(&profile);
    let r = Some("22BCE0001");
    assert_eq!(confirm_from(Some("a.22bce0001@vitapstudent.ac.in"), r, None), ConfirmFrom::CollegeInbox);
    assert_eq!(confirm_from(Some(" Ann <A.22BCE0001@VITAPSTUDENT.AC.IN> "), r, None), ConfirmFrom::CollegeInbox);
    assert_eq!(confirm_from(Some("odd.name@vitapstudent.ac.in"), None, p), ConfirmFrom::CollegeInbox);
    assert_eq!(confirm_from(Some("b.22bce0002@vitapstudent.ac.in"), r, p), ConfirmFrom::Unknown);
    assert_eq!(confirm_from(Some("me@gmail.com"), r, p), ConfirmFrom::OwnOther);
    assert_eq!(confirm_from(Some("someone@gmail.com"), r, p), ConfirmFrom::Unknown);
    assert_eq!(confirm_from(Some("someone@gmail.com"), r, None), ConfirmFrom::Unknown);
    assert_eq!(confirm_from(None, r, p), ConfirmFrom::NotSaid);
    assert_eq!(confirm_from(Some("  "), r, p), ConfirmFrom::NotSaid);
}
```

- [ ] **Step 2: Run it and check that it fails**

Run: `cd rust && cargo test -p vtop-ffi tells_who_confirmed`
Expected: a compile error, because `confirm_from` and `ConfirmFrom` aren't defined.

- [ ] **Step 3: Implement `confirm_from`**

Take the address out of the requester: if there's a `<…>`, use what's inside, then trim
and lowercase it. An empty result gives `NotSaid`. Then:
- `CollegeInbox` when the domain is `vitapstudent.ac.in` and the part before the `@`
  contains the lowercased regno, or when it equals the profile's `vit_email` (ignoring
  case).
- `OwnOther` when it matches any word in the profile's fields, using the same split as
  today's `requester_known`.
- `Unknown` otherwise.

In `forwarding_view`, set `from` with `regno = self.student()` (the username is the
regno) and the profile read the way `requester_known` reads it today. Delete
`requester_known`. Where the confirmation is built (`From<StoredConfirmation>` and
`Setup::mail`), set `from: ConfirmFrom::NotSaid`; `forwarding_view` always overwrites
it.

- [ ] **Step 4: Run the forwarding tests and check that they pass**

Run: `cd rust && cargo test -p vtop-ffi forwarding`
Expected: all pass, including `tells_who_confirmed`. Update the
`requester_known: None` literals in tests to `from: ConfirmFrom::NotSaid`.

---

### Task 2: Step, hint and the side file

**Files:**
- Modify: `rust/vtop-ffi/src/remote/forwarding.rs`: `Forwarding` (around lines
  196–226), `view` (around line 1175), `forget_forwarding`, plus new exports in the
  `#[uniffi::export] impl Core` block
- Test: `rust/vtop-ffi/src/remote/forwarding/tests.rs`

**Interfaces:**
- Consumes: `ConfirmFrom` from Task 1.
- Produces:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)] pub enum SetupStep { AddAddress, Confirm, Filter }`
  - `#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)] pub enum ForwardingHint { TypedOtp { at: i64 } }`
  - `Forwarding::Waiting` gains `step: SetupStep`. `Forwarding::On` gains
    `hint: Option<ForwardingHint>`.
  - Both `Waiting` and `On` gain `college_mail: Option<String>`: the saved profile's
    `vit_email` when there is one, else `None`. It's never guessed from the regno.
    Kotlin uses it for the account chip and `authuser`.
  - Exports, none of them using the network, each returning `Forwarding`:
    - `pub fn opened_forwarding_confirm(&self, flow: String) -> Forwarding`
    - `pub fn note_typed_otp(&self, flow: String, at: i64) -> Forwarding`
    - `pub fn dismiss_forwarding_hint(&self, flow: String) -> Forwarding`
  - The side file:
    `const PROGRESS_FILE: &str = "forwarding_progress.json";`
    `#[derive(Serialize, Deserialize, Default)] struct Progress { id: String, confirm_opened_at: Option<i64>, typed_otp_at: Option<i64>, dismissed_at: Option<i64> }`.
    This one has no `deny_unknown_fields`, so it can grow.

**Rules, all pure, in `view(state, progress)`:**
- Progress whose `id` isn't the current mailbox id is treated as `Progress::default()`.
- `step`:
  - Readiness `NeedsConfirmation` gives `AddAddress`.
  - Readiness `NeedsFilter` with a confirmation whose `sent_at` is newer than
    `confirm_opened_at` (or with nothing opened) gives `Confirm`.
  - Any other `NeedsFilter` gives `Filter`. That includes no confirmation at all, which
    is mail seen without one.
- `hint`: `Some(TypedOtp { at })` when `typed_otp_at = Some(at)` and
  `at > max(last_otp_at, dismissed_at)`. Otherwise `None`.
- `note_typed_otp` records only when the state is `Joined` with `Setup::Active`;
  otherwise it does nothing.
- `opened_forwarding_confirm` sets `confirm_opened_at` to the shown confirmation's
  `sent_at`.
- `dismiss_forwarding_hint` sets `dismissed_at = typed_otp_at`.
- `forget_forwarding` removes `PROGRESS_FILE`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn step_follows_gmail_mail_and_the_confirm_tap() {
    let fresh = MailboxState::Joined { mailbox: mailbox(ID), setup: Setup::NEW };
    assert_eq!(step(&view(&fresh, &Progress::default())), Some(SetupStep::AddAddress));
    let confirmed = fresh.clone().with_setup(ID, |s| s.mail(&confirm(100)));
    assert_eq!(step(&view(&confirmed, &Progress::default())), Some(SetupStep::Confirm));
    let opened = Progress { id: ID.into(), confirm_opened_at: Some(100), ..Progress::default() };
    assert_eq!(step(&view(&confirmed, &opened)), Some(SetupStep::Filter));
    // A newer confirmation (added again in Gmail) asks again.
    let again = confirmed.with_setup(ID, |s| s.mail(&confirm(200)));
    assert_eq!(step(&view(&again, &opened)), Some(SetupStep::Confirm));
    // Another address's progress doesn't count.
    let other = Progress { id: "other".into(), confirm_opened_at: Some(999), ..Progress::default() };
    assert_eq!(step(&view(&again, &other)), Some(SetupStep::Confirm));
}

#[test]
fn the_typed_otp_hint_never_changes_the_state() {
    let on = MailboxState::Joined { mailbox: mailbox(ID), setup: Setup::Active { last_mail_at: Some(50), last_otp_at: Some(50) } };
    let typed = Progress { id: ID.into(), typed_otp_at: Some(60), ..Progress::default() };
    let v = view(&on, &typed);
    assert_eq!(hint(&v), Some(ForwardingHint::TypedOtp { at: 60 }));
    assert_eq!(readiness(&v), ForwardingReadiness::Ready);
    // A forwarded OTP after it, or a dismiss, hides it; a later typed one shows again.
    assert_eq!(hint(&view(&on.clone().with_setup(ID, |s| s.otp_at(70)), &typed)), None);
    let dismissed = Progress { dismissed_at: Some(60), ..typed.clone() };
    assert_eq!(hint(&view(&on, &dismissed)), None);
    let later = Progress { typed_otp_at: Some(80), ..dismissed };
    assert_eq!(hint(&view(&on, &later)), Some(ForwardingHint::TypedOtp { at: 80 }));
}

#[tokio::test]
async fn progress_lives_beside_forwarding_json_and_goes_with_it() {
    let (core, edge, dir, _url) = forwarding_core("progress").await;
    core.set_up_forwarding("f".into()).await.unwrap();
    // Not set up yet: a typed OTP is not noted.
    assert_eq!(hint(&core.note_typed_otp("f".into(), 10)), None);
    seal(&core, &edge, json!({ "kind": "gmail_confirm", "code": "1", "link": null, "requester": null, "sentAt": 5 }));
    core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(step(&core.opened_forwarding_confirm("f".into())), Some(SetupStep::Filter));
    core.mark_forwarding_done("f".into());
    assert_eq!(hint(&core.note_typed_otp("f".into(), 20)), Some(ForwardingHint::TypedOtp { at: 20 }));
    assert_eq!(hint(&core.dismiss_forwarding_hint("f".into())), None);
    // forwarding.json still parses as the old Stored (an older app reads it).
    let stored: Stored = serde_json::from_slice(&fs::read(dir.join(FILE)).unwrap()).unwrap();
    assert!(stored.ready);
    assert!(dir.join(PROGRESS_FILE).exists());
    core.turn_off_forwarding("f".into()).await.unwrap();
    assert!(!dir.join(PROGRESS_FILE).exists());
}
```

Add the helpers `step(&Forwarding) -> Option<SetupStep>` (from `Waiting`) and
`hint(&Forwarding) -> Option<ForwardingHint>` (from `On`) next to the existing
`readiness` helper. If `forwarding_core` writes student files somewhere other than
`dir`, point the file checks there; it's the same root `FILE` uses.

- [ ] **Step 2: Run them and check that they fail**

Run: `cd rust && cargo test -p vtop-ffi forwarding`
Expected: compile errors, because `Progress`, `SetupStep` and the new exports don't
exist yet.

- [ ] **Step 3: Implement the side file, the rules and the exports**

Read and write the side file under the existing `locks.forwarding` lock, with
`self.read` and `self.write`. `forwarding_status` and every action pass
`self.progress()` to `view`. Log `"typed otp noted"`, `"hint dismissed"` and
`"confirm link opened"` at `info`, with no values.

- [ ] **Step 4: Run all of the crate's checks**

Run: `cd rust && cargo fmt && cargo clippy -p vtop-ffi --all-targets -- -D warnings && cargo test -p vtop-ffi`
Expected: all pass, including `stored_files_of_every_version_read` without changes.

- [ ] **Step 5: Rebuild the bindings**

Run: `mise run rust-android-dev`
Expected: it succeeds, and `core/src/vitapmate/core/vtop_ffi.kt` has `ConfirmFrom`,
`SetupStep`, `ForwardingHint`, `openedForwardingConfirm`, `noteTypedOtp` and
`dismissForwardingHint`.

---

### Task 3: Tag the guide's blocks with steps, and group them in Kotlin

**Files:**
- Modify: `backend/guides/forwarding.json`: add `"step"` to each `step` block (1-add
  gets 1, 2-verify gets 2, 4-noreload, 3-from, 5-forward and 6-done get 3). The first
  callout gets no step.
- Create: `features/login/src/vitapmate/features/login/GuidePages.kt`
- Test: `features/login/test/vitapmate/features/login/GuidePagesTest.kt`
- Modify: `features/login/test/vitapmate/features/login/ForwardingPhaseTest.kt`, so
  its constructors take the new `step` and `hint` arguments

**Interfaces:**
- Produces:
  - `internal data class GuidePage(val step: Int?, val title: String, val body: String, val note: GuideNote?, val shot: String?, val caption: String?)`
  - `internal data class GuideNote(val icon: String, val tone: String, val text: String)`
  - `internal fun pagesFor(step: Int, pages: List<GuidePage>): List<GuidePage>`. This
    returns the pages tagged `step`, in order. Untagged pages are never returned.
  - `internal fun guidePages(json: JSONObject, sender: String): List<GuidePage>`. This
    parses step blocks and replaces `{sender}`; it's the existing parsing moved here.
    It isn't unit-tested, because `org.json` is a stub in JVM tests.

- [ ] **Step 1: Write the failing test**

```kotlin
class GuidePagesTest {
    private fun page(step: Int?, title: String) = GuidePage(step, title, "", null, null, null)

    @Test
    fun pagesForKeepsOrderAndSkipsUntagged() {
        val all = listOf(page(1, "add"), page(null, "old"), page(3, "reload"), page(2, "verify"), page(3, "from"))
        assertEquals(listOf("reload", "from"), pagesFor(3, all).map { it.title })
        assertEquals(listOf("add"), pagesFor(1, all).map { it.title })
        assertEquals(emptyList(), pagesFor(4, all))
    }
}
```

- [ ] **Step 2: Run it and check that it fails**

Run: `./kotlin test`
Expected: `GuidePagesTest` fails to compile, because `GuidePage` isn't defined.

- [ ] **Step 3: Implement `GuidePages.kt` and edit `forwarding.json`**

Add only the `"step"` keys, with no other change to the JSON. Check it with
`bunx tsc -p backend --noEmit`, since `guides.ts` imports it.

- [ ] **Step 4: Run the tests and check that they pass**

Run: `./kotlin test`
Expected: all pass, including the updated `ForwardingPhaseTest`.

---

### Task 4: Gmail links that open the right account in Chrome

**Files:**
- Create: `features/login/src/vitapmate/features/login/GmailLinks.kt`
- Test: `features/login/test/vitapmate/features/login/GmailLinksTest.kt`

**Interfaces:**
- Produces:
  - `internal enum class GmailPage(val fragment: String) { Forwarding("settings/fwdandpop"), Filters("settings/filters") }`
  - `internal fun gmailUrl(page: GmailPage, account: String?): String`. With an account,
    this gives `https://mail.google.com/mail/?authuser=<url-encoded account>#<fragment>`.
    Without one, `https://mail.google.com/mail/u/0/#<fragment>`.
  - `internal fun prefilledFilterUrl(sender: String, account: String?): String`. Same
    base, with fragment `create-filter/from=<url-encoded sender>`.
  - `internal fun openInChrome(context: Context, url: String)`. This uses an
    `ACTION_VIEW` intent with `setPackage("com.android.chrome")`. On
    `ActivityNotFoundException`, it sends the same intent without a package.

- [ ] **Step 1: Write the failing test**

```kotlin
class GmailLinksTest {
    @Test
    fun urlsCarryTheAccountAndPage() {
        assertEquals("https://mail.google.com/mail/?authuser=a.22bce0001%40vitapstudent.ac.in#settings/fwdandpop", gmailUrl(GmailPage.Forwarding, "a.22bce0001@vitapstudent.ac.in"))
        assertEquals("https://mail.google.com/mail/u/0/#settings/filters", gmailUrl(GmailPage.Filters, null))
        assertEquals("https://mail.google.com/mail/u/0/#create-filter/from=noreply.sdc%40vitap.ac.in", prefilledFilterUrl("noreply.sdc@vitap.ac.in", null))
    }
}
```

- [ ] **Step 2: Run it, check that it fails, implement, then run it again and check that it passes**

Run: `./kotlin test`
Expected: first a compile failure, then a pass. Use
`java.net.URLEncoder.encode(s, "UTF-8")`.

- [ ] **Step 3: Check the links on the emulator**

Open both URLs on `emulator-5554` with Chrome signed in to a Google account:

```
adb -s emulator-5554 shell am start -a android.intent.action.VIEW -p com.android.chrome -d '<url>'
```

Note whether `authuser` picks the account and whether the `create-filter` fragment
opens a prefilled filter. If the prefilled filter doesn't work, Task 5's "Open the
filter" uses `gmailUrl(GmailPage.Filters, …)` instead. Record the result in the task
report.

---

### Task 5: The steps sheet and "Show me"

**Files:**
- Modify: `features/login/src/vitapmate/features/login/ForwardingSetup.kt`
- Create: `features/login/src/vitapmate/features/login/GuideSheet.kt`. This holds the
  `Screenshot` and `ShotViewer` code moved out of `ForwardingGuide.kt`.
- Delete: `features/login/src/vitapmate/features/login/ForwardingGuide.kt`
- Modify: `features/login/src/vitapmate/features/login/MailForwarding.kt`: add
  `ForwardingOp.Confirm("confirm")` and `ForwardingOp.Dismiss("dismiss")`, and
  `fun openedConfirm()` and `fun dismissHint()`, run like `markDone`

**Interfaces:**
- Consumes:
  - From Task 2: `Forwarding.Waiting.step`, `Forwarding.On.hint`, `ConfirmFrom` and
    the three exports.
  - From Task 3: `guidePages`, `pagesFor`.
  - From Task 4: `gmailUrl`, `prefilledFilterUrl`, `openInChrome`.
- Produces: `@Composable internal fun GuideSheet(step: Int, title: String, address: String, sender: String, onClose: () -> Unit)`.
  This is a `ModalBottomSheet` that loads `rememberGuide("forwarding.json")`, pages
  through `pagesFor(step, …)` with Back and Next and dots, and shows copy chips for From
  and Forward to. Test tags: `forwarding.guide`, `forwarding.guide.shot`,
  `forwarding.guide.next`.

What to draw (screens 1, 2, 4, 5 and 7 of the spec):
- **The address card.** As it is now, with a new pill text:
  - `READY`: "On · last OTP {ago}", or "On · no OTP forwarded yet" when there's none.
  - `NEEDS_FILTER` at `Confirm`: "Confirm in Gmail".
  - `NEEDS_FILTER` at `Filter`: "Create the filter".
  - `NEEDS_CONFIRMATION`: "Waiting for Gmail's confirmation…".
- **The progress bar.** Three segments and "n of 3", from `step`. `On` shows "3 of 3",
  all done. Test tag `forwarding.progress`.
- **Step 1.** College-account chip (`college_mail` from Rust; no chip and no
  `authuser` when it's null),
  "Open in Chrome" (`openInChrome(gmailUrl(Forwarding, account))`) and "Show me"
  (`GuideSheet(1, …)`).
- **Step 2.** When there's a confirmation:
  - `CollegeInbox`, `OwnOther` or `NotSaid`: a success Callout (warning tone for
    `OwnOther`) saying "From your college Gmail" plus the requester. Its main button is
    "Confirm in Gmail": open the link with `openInChrome`, then
    `MailForwarding.openedConfirm()`. Show "Use the code" only for `ConfirmBy.Code`,
    which reveals the code row with its copy button (test tag
    `forwarding.confirm_code`).
  - `Unknown` or `OwnOther`: the wrong-account card from screen 5. It has "That's not
    your college Gmail" plus the requester, the line "VTOP sends OTPs to your college
    mail, so this Gmail never gets them.", "Open college Gmail" (opens
    `gmailUrl(Forwarding, college account)`), a text button "It's right, confirm
    anyway" that reveals the normal confirm action, and a lab-tone note to remove the
    address from the other Gmail. Test tag `forwarding.wrong_account`.
- **Step 3.** "Open the filter" (prefilled if Task 4 says it works), "Show me"
  (`GuideSheet(3, …)`) and "I've created it" (`forwarding.done`, shown at `Filter`).
  After it, the card "That's it. Forwarding is On after this step. Nothing waits for
  VTOP to send an OTP."
- **Hint** (`On` with a hint): a warning Callout saying "Typed the OTP yourself at
  {HH:mm}? It may not have been forwarded. Worth a quick look at the filter." It has a
  Dismiss button (`MailForwarding.dismissHint()`) and a 4-row checklist (From exactly
  `{sender}`; forwards to your address; in your college Gmail; forwarding still
  verified), each row opening `gmailUrl(Filters, …)` or `gmailUrl(Forwarding, …)`.
  Test tag `forwarding.hint`.
- Remove the old "Step-by-step guide with screenshots" button and the From row; the
  sender now lives in the Show me chips.

- [ ] **Step 1: Build**

Run: `./kotlin build`
Expected: BUILD SUCCESSFUL.

- [ ] **Step 2: Lint**

Run: `mise run detekt`
Expected: no new findings in `features/login`.

- [ ] **Step 3: Check on the emulator with the demo account**

Use the `test-on-emulator` skill. Sign in as the demo account, open Settings → Account
→ Mail forwarding, and capture `mise run ui-dump`. Expected: tags `forwarding.progress`
and `forwarding.step1` are present. Opening "Show me" shows `forwarding.guide` with a
`forwarding.guide.shot`.

The demo account answers forwarding as unavailable. If that blocks the steps, check
the steps with a local Convex (Settings → Developer → Server), or report that only the
intro could be checked.

---

### Task 6: Note OTPs typed by hand

**Files:**
- Modify: `features/login/src/vitapmate/features/login/OtpChallenge.kt` (around lines
  163–200 and 270–305)

**Interfaces:**
- Consumes: `noteTypedOtp(flow, at)` from Task 2.

- [ ] **Step 1: Implement**

Keep `private val autofilled = mutableSetOf<String>()` and add each code `watch` fills
in. In `submit`, after `accepted = true`, when all of these hold:
- `code !in autofilled`
- `OtpAutofill.inUse() == OtpSource.Forwarding`
- `MailForwarding.readiness == ForwardingReadiness.READY`

then call `core.noteTypedOtp(lastFlow, System.currentTimeMillis() / 1000)` and refresh
the status with `MailForwarding.load("typed_otp")` on `AppScope`. Log
`"otp typed while forwarding"` under `src`, without the code.

- [ ] **Step 2: Build and check on the emulator**

Run: `./kotlin build`. Then, on the emulator signed in with forwarding `On` (local
Convex), type an OTP by hand. Expected: `mise run state` shows the forwarding hint, and
the sheet shows `forwarding.hint`. If no real OTP prompt can be reached, report that the
hint was checked only through Task 2's Rust tests.

---

### Task 7: Turn off and Delete clean up Gmail

**Files:**
- Modify: `features/login/src/vitapmate/features/login/ForwardingContent.kt` (the
  `confirmOff` dialog, around lines 87–102)
- Modify: `features/login/src/vitapmate/features/login/ForwardingPhones.kt` (the
  delete dialog, around lines 131–145)
- Create: `features/login/src/vitapmate/features/login/GmailCleanup.kt`

**Interfaces:**
- Consumes: `gmailUrl`, `openInChrome` from Task 4.
- Produces: `@Composable internal fun GmailCleanup(sender: String, address: String)`.
  This is the two-step card from screen 8, drawn with `Step(…)`:
  1. "Delete the filter": "Filters → the one for {sender} → Delete", with an "Open
     filters" link.
  2. "Remove the address": "Forwarding → pick {address} → Remove address", with an
     "Open forwarding" link.

  Test tag `forwarding.cleanup`.

- [ ] **Step 1: Implement**

Swap both confirm dialogs for a `ModalBottomSheet` laid out like screen 8:
- A danger icon.
- Title "Turn off forwarding?" (or the existing delete title).
- One line of what happens.
- `GmailCleanup`.
- The danger confirm button and Cancel.

Keep `confirmTag = "forwarding.off.confirm"` and the delete confirm's existing tag.

- [ ] **Step 2: Build, lint and check on the emulator**

Run: `./kotlin build && mise run detekt`. Then tap Turn off on the emulator. Expected:
`forwarding.cleanup` and `forwarding.off.confirm` show in the UI dump. Cancel leaves
forwarding unchanged.

---

### Task 8: Full check

- [ ] **Step 1:** Run `mise run check-rust && mise run check-kotlin && mise run check-backend`. Expected: all pass.
- [ ] **Step 2:** Use `mise run check-changed` to make sure nothing else was missed.
- [ ] **Step 3:** In the report, list what was checked on the emulator, what couldn't
  be checked and why, and Task 4's results for `authuser` and the prefilled filter
  link.
