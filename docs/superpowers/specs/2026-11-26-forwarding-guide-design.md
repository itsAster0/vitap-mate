# Mail forwarding guide redesign

## Why

Setting up mail forwarding is the hardest thing the app asks a student to do. Today the
sheet has 3 steps and the guide has 6 others, with different numbers and different
advice. Most of the work happens in Gmail's desktop site, and the student never learns
about the failures that happen without any sign:

- Setting it up in the wrong Gmail (a personal one instead of the college one).
- Gmail forwarding all mail instead of only what the filter matches.
- A filter that never forwards anything.

The goal is a setup that most students finish in one go and that tells them when
something is wrong, without the app ever claiming more than it can know.

## Goals

- One list of steps. The guide's screenshots sit on the steps they belong to.
- Progress the app can stand behind. Setup is driven only by Gmail's confirmation mail
  and the student's own taps.
- The common mistakes are named in plain words, where they happen.
- Fewer trips between apps: confirming from a notification, links that open the right
  Gmail account, copy buttons where they're needed.
- A laptop path: the same guide on a bigger screen.

## Non-goals

- Doing the setup automatically through the Gmail API. Its restricted scope needs
  Google verification and a paid security assessment.
- An in-app Gmail web view. Google blocks sign-in inside web views.
- Changes to how codes are sealed, relayed or kept.

## Anti-goals

- **Nothing waits for, or is locked by, a VTOP OTP.** VTOP may never send one, or not for
  weeks. Forwarding is On once the filter step is done. A forwarded OTP only adds "last
  OTP …" to the status. Background sync keeps its current rule (allowed from the
  "needs filter" stage on, `forwarding.rs:189`).
- No status claims "working" when nothing proves it.
- No change of look outside the forwarding screens. M3 Expressive and theme colours
  only.

## Constraints

- Root `AGENTS.md` rules hold. Domain state and calculations live in Rust and cross
  UniFFI as enums. Kotlin only draws them. Every action is a `flow()`. Nothing logs
  codes, links or addresses' keys.
- `backend/guides/forwarding.json` stays readable by older apps. New fields are added;
  blocks are not removed or renumbered.
- Server state is in Convex and watched live. The mail Worker keeps nothing.
- Prod deploys (Convex, Workers) and a Play release each need an explicit OK.
- New dependencies need an OK. A QR code library is needed for the laptop handoff
  (see Open points).

## Approaches considered

1. **Merge the two lists into the sheet's steps** (picked). The steps are driven by
   the readiness Rust already gives, and each step's screenshots open in a sheet. One
   source of truth, and the screenshots are kept.
2. **Keep the separate full-screen guide and make it react to readiness.** Two lists
   would still drift apart, and the guide would cover the sheet's buttons.
3. **A full-screen wizard, one screen per step.** It hides the overview, and the
   student loses their place when they come back from Chrome.

For the extension (since dropped, see Dropped):

1. **It guides inside Gmail** (picked). A floating card points at the next control,
   fills in the address and the From field, and moves on as the page changes. If it
   can't find a control, it falls back to the guide. The student makes every click.
2. **It clicks through Gmail itself.** Too fragile: Gmail's page structure is
   obfuscated, adding an address opens a popup, and Gmail may ask for the password
   again.
3. **Gmail API.** Ruled out (see Non-goals).

## The design

The mockups are screens 1–10 from the 2026-11-26 conversation. They are summarised
here.

### Part 1: the app (Rust and Kotlin)

**Steps (screens 1, 2, 4).** The sheet keeps its address card and status pill. Below
them are a progress bar ("n of 3") and three steps:

1. **Add the address in Gmail.** Shows a college-account chip, an "Open in Chrome"
   button and a "Show me" button.
2. **Confirm it.** Done in one tap, from here or from the notification.
3. **Create the filter.** Shows "Open the filter", "Show me" and "I've created it".

Then a note: forwarding is On after step 3, and nothing waits for VTOP.

- Step 1 ticks when Gmail's confirmation mail arrives.
- Step 2 ticks when the student opens the confirm link (from the app or the
  notification) or taps "I've confirmed". The app can't see the click in Gmail, so
  the student's tap counts.
- Step 3 ticks when the student taps "I've created it", as today.

**Confirmation (screen 4).** The note says whose Gmail asked. Its main action is
"Confirm in Gmail", which opens the link. "Use the code" opens the code only when the
mail has no link (`ConfirmBy::Code`).

**Who confirmed.** `requester_known: Option<bool>` becomes an enum in Rust:

- `CollegeInbox`: the requester is on `@vitapstudent.ac.in` and the part before the
  `@` contains the signed-in regno (ignoring case), or it equals `vit_email` exactly.
- `OwnOther`: an address found elsewhere on the saved profile.
- `Unknown`: any other address.
- `NotSaid`: the mail named no address.

**Wrong account (screen 5).** `OwnOther` and `Unknown` show a warning card with a small
diagram (VTOP → college Gmail, with the personal Gmail crossed out). The main action is
"Open college Gmail". A quieter "It's right, confirm anyway" is still offered. There is
also a note to remove the address from the other Gmail.

**Show me (screen 2).** A bottom sheet with the step's screenshots as pages. Each page
has its caption and note, copy chips for From and Forward to, Back and Next, and the
live status pill. `forwarding.json` blocks get an optional `"step": 1|2|3` field and an
optional `"phase"` label. Blocks without `step` stay as they are, so older apps still
show the full list.

**Links.**
- Open in Chrome explicitly (an intent with Chrome's package, falling back to the
  default browser), so the Gmail app doesn't grab them.
- Add `?authuser=<college mail>` so the right account opens.
- Try the prefilled filter link (`#create-filter/from=<sender>`) on the emulator. If
  it doesn't work, use the plain filters link.

**The typed-OTP hint (screen 7).** When the student submits an OTP by hand while the
address is set up and nothing was forwarded during that prompt, Rust records
`manual_otp_at`. The sheet then shows a warning-tone hint that can be dismissed
("Typed the OTP yourself at 9:41? It may not have been forwarded…"), with a checklist
of what to check in Gmail. A forwarded OTP or Dismiss clears it. It changes no state,
pauses nothing and has no effect on background sync.

**Turn off (screen 8).** Turn off and Delete open a confirmation sheet with the two
Gmail clean-up steps and their links (delete the filter, remove the address) before
going ahead.

### Part 2: the server (Convex, the mail Worker, the app's push handling)

**Confirmation notification (screen 3).** `mail:deliver` already sends a `mail_otp`
push with the sealed box for a `gmail_confirm`. The app's `PushService` opens it
through Rust. For a confirmation, when the forwarding screen isn't in front, it posts
a notification: "Gmail's confirmation arrived". It has a **Confirm** action that opens
the link in Chrome and marks step 2, and an "Open app" action. If there's no link, the
notification shows the code instead.

**Forwarding all mail (screen 6).** The Worker drops anything that isn't VTOP's OTP
mail or Gmail's confirmation. For an address that exists, it now also calls a new
internal Convex mutation that counts dropped mails per address per week, with no
content or sender kept. `mail:inbox` returns `otherMail: { count, lastAt }`. Rust turns
that into a warning, and the sheet shows "Gmail is forwarding all your mail" with a
link to forwarding settings.

The status label reads "On · last OTP …" or "On · no OTP forwarded yet".

### Part 3: laptop handoff (screens 9, 10)

- "Easier on a laptop" calls a new mutation, `mail:handoff`. It returns a token for
  this address that works for 30 minutes.
- The phone shows a QR code and the link `vmate.aster0.dev/f/<token>`.
- A Convex http route serves the guide page for the token. The page polls a small
  state endpoint for the address, sender, college mail and the step reached, so the
  phone and the laptop tick together.
- The page has the same steps, screenshots and copy chips as the app.
- Tokens are single-purpose and expire. The page shows no code and no confirm link;
  those stay on the phone.

### Later

Short looping clips to replace the filter screenshots, recorded in Gmail. The `shot`
field gets an optional `clip`.

## Dropped

- The extension guide ("Guide me in Gmail"): dropped on 2026-11-26. Option 3 (guiding
  inside Gmail) was the pick had it gone ahead.

## Order

1. Part 1, the app. No server change. It ships with an app release.
2. Part 2. Convex and Worker changes plus the app's push handling. Needs a deploy OK.
3. Part 3. Needs a deploy OK and the QR dependency OK.

## Decided by me, open to correction

- Assumed: VTOP sends OTPs to the college mail. If that's wrong, `OwnOther` gets a
  milder note.
- A wrong account gets a warning but isn't blocked.
- Step 2 ticks when the student opens the confirm link, since Gmail sends nothing after
  the click.
- The dropped-mail count is per week, a number only.
- The handoff token lasts 30 minutes.

## Left to the builder

- The exact Rust names (`ConfirmFrom`, `ForwardingHint`), the shape of the state, and
  the record migration for `manual_otp_at`.
- How the Show me sheet pages through screenshots, which Compose pager to use.

## Compatibility

- Every backend change is an addition. The current app, the current extension and the
  current server routes keep working.
- The `"step"` field in `guides/forwarding.json` is ignored by older apps, which keep
  showing the full list. The existing blocks and wording stay valid on their own.
- `mail:inbox`'s new `otherMail` field is ignored: Rust reads the reply as a
  `serde_json::Value`.
- Deploy order for Part 2: Convex first, then the Worker. The Worker ignores a failed
  counting call, so delivery never depends on it.
- The phone's own `forwarding.json` refuses unknown fields (`Stored`, with
  `deny_unknown_fields`), and older apps read it. New local state (`manual_otp_at`,
  confirm link opened, hint dismissed) goes in a separate file next to it.

## Decided

- The QR code comes from the `qrcode` crate in `vtop-ffi`. Rust returns the module
  grid, and Compose draws it in theme colours. This dependency is approved.

## Open points

- Whether the prefilled filter link and `authuser` work. Checked on the emulator in
  Part 1.
