#!/bin/sh
# Builds corpus/parsers from the parser tests' fixtures: each file prefixed with the byte
# that picks its parser in fuzz_targets/parsers.rs (PARSERS). Run from this folder.
set -eu
fixtures=../tests/fixtures
out=corpus/parsers
mkdir -p "$out"
seed() { # seed <selector> <name> <file>...
  sel=$1 name=$2
  shift 2
  { printf "\\$(printf '%03o' "$sel")"; cat "$@"; } > "$out/seed-$name"
}
seed 0 semesters "$fixtures/semesters.html"
seed 1 attendance "$fixtures/attendance.html"
seed 2 full_attendance "$fixtures/full_attendance.html"
seed 3 timetable "$fixtures/timetable.html"
seed 4 marks "$fixtures/marks.html"
seed 5 exam_schedule "$fixtures/exam_schedule.html"
seed 6 grade_view "$fixtures/grade_view.html"
seed 7 calendar_months "$fixtures/calendar_months.html"
seed 8 calendar_month "$fixtures/calendar_month.html"
seed 9 grade_details "$fixtures/grade_details.html"
seed 10 grade_history "$fixtures/grade_history.html"
seed 11 biometric "$fixtures/biometric.html"
seed 12 profile "$fixtures/profile.html"
{ printf '\015'; cat "$fixtures/mentor_details.html"; printf '\000'; cat "$fixtures/mentor_messages.html"; } > "$out/seed-mentor"
# 14..=22 have no fixture; the HTML pages above carry their tags and tables.
printf '\026{"internalDate":"1","snippet":"123456","payload":{"headers":[{"name":"From","value":"VTOP <noreply.sdc@vitap.ac.in>"}],"body":{"data":"PGI-MTIzNDU2PC9iPg"}}}' > "$out/seed-gmail"
{ printf '\027data:image/jpeg;base64,'; base64 < ../tests/captchas/1.jpg | tr -d '\n'; } > "$out/seed-captcha-url"
for jpg in ../tests/captchas/*.jpg; do
  seed 24 "captcha-$(basename "$jpg" .jpg)" "$jpg"
done
echo "seeded $(ls "$out" | grep -c "^seed-") inputs in $out"
