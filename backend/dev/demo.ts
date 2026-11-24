// Turns a demo recording (`demo_record.json`, see rust/vtop-ffi/src/demo.rs) into
// guides/demo.json with a made-up student: the profile, mentor, outing and grade history
// identity is replaced, and every personal value the profile held is swept from all pages.
// Fails when one is still there. Review the output, then `bun dev/gen-guides.ts` and `bunx convex deploy`.
//
//   bun dev/demo.ts <demo_record.json>

import { readFileSync, writeFileSync } from "node:fs";

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type Obj = { [key: string]: Json };
type Field = { label: string; value: string };

const FAKE = {
  name: "AARAV SHARMA",
  regno: "23BCE9999",
  email: "aarav.23bce9999@vitapstudent.ac.in",
  phone: "9000000000",
  mentor: "Dr. Meera Rao",
  mentorEmail: "meera.rao@vitap.ac.in",
};

/** A made-up value for a profile or mentor field, by its label; anything not listed is blanked. */
function fake(label: string): string {
  const l = label.toUpperCase();
  const rules: [RegExp, string][] = [
    [/FACULTY NAME/, FAKE.mentor],
    [/FACULTY EMAIL/, FAKE.mentorEmail],
    [/STUDENT NAME/, FAKE.name],
    [/FATHER|MOTHER|GUARDIAN NAME/, "RAJESH SHARMA"],
    [/EMAIL/, FAKE.email],
    [/MOBILE|PHONE|WHATSAPP|INTERCOM|CONTACT/, FAKE.phone],
    [/REGISTER|ROLL/, FAKE.regno],
    [/DATE OF BIRTH/, "01-Jan-2005"],
    [/GENDER/, "Male"],
    [/BLOOD/, "O+"],
    [/NATIONALITY|COUNTRY/, "Indian"],
    [/STATE/, "Andhra Pradesh"],
    [/CITY|DISTRICT|AREA|STREET/, "Amaravati"],
    [/PIN|ZIP/, "522237"],
    [/BLOCK/, "MH-1"],
    [/ROOM/, "101"],
    [/HOSTELLER/, "Yes"],
    [/LANGUAGE/, "Telugu"],
    [/DESIGNATION/, "Assistant Professor"],
    [/DEPARTMENT/, "SCOPE"],
    [/CABIN/, "AB1-101"],
  ];
  return rules.find(([pattern]) => pattern.test(l))?.[1] ?? "—";
}

const record = process.argv[2];
if (!record) throw new Error("usage: bun dev/demo.ts <demo_record.json>");
const bundle = JSON.parse(readFileSync(record, "utf8")) as { schema: number; pages: Obj };
const pages = bundle.pages;

// Every personal value to sweep from the other pages: the profile's, the mentor's and the outing student's.
const secrets = new Map<string, string>();
/** Labels whose values identify someone, and so are swept everywhere. */
const PERSONAL = /NAME|EMAIL|MOBILE|PHONE|WHATSAPP|INTERCOM|REGISTER|ROLL|APPLICATION|AADHAR|BIRTH|STREET|AREA|PIN|ZIP|ADDRESS|EMP\.ID|FACULTY ID|CABIN/i;
const walkFields = (node: Json, visit: (field: Field) => void) => {
  if (Array.isArray(node)) node.forEach((n) => walkFields(n, visit));
  else if (node && typeof node === "object") {
    if (typeof node.label === "string" && typeof node.value === "string") visit(node as Field);
    Object.values(node).forEach((n) => walkFields(n, visit));
  }
};
const keep = (value: unknown, replacement = "—") => {
  if (typeof value === "string" && value.trim().length >= 4 && !secrets.has(value.trim())) secrets.set(value.trim(), replacement);
};

const profile = pages.profile as Obj | undefined;
if (!profile) throw new Error("record the profile page first (open Profile)");
keep(profile.vit_email, FAKE.email);
keep(profile.registration_number, FAKE.regno);
keep(profile.name, FAKE.name);
String(profile.name).split(/\s+/).filter((part) => part.length >= 4).forEach((part) => keep(part, FAKE.name));
walkFields(profile, (f) => {
  if (PERSONAL.test(f.label)) keep(f.value, fake(f.label));
  f.value = fake(f.label);
});
Object.assign(profile, { name: FAKE.name, registration_number: FAKE.regno, vit_email: FAKE.email, photo: "" });

const mentor = pages.mentor as Obj | undefined;
if (mentor) {
  walkFields(mentor, (f) => {
    if (PERSONAL.test(f.label)) keep(f.value, fake(f.label));
    f.value = fake(f.label);
  });
  mentor.messages = [];
}

for (const name of ["general_outing", "weekend_outing"]) {
  const student = (pages[name] as Obj | undefined)?.student as Obj | null | undefined;
  if (!student) continue;
  keep(student.application_no);
  keep(student.parent_contact_number, FAKE.phone);
  Object.assign(student, {
    registration_number: FAKE.regno, name: FAKE.name, application_no: "2023000000", gender: "Male",
    hostel_block: "MH-1", room_number: "101", parent_contact_number: FAKE.phone,
  });
}

// Why the student went out says too much (a doctor's visit): ordinary reasons instead.
const PURPOSES = ["Shopping", "Movie", "Visiting family", "Going home"];
for (const name of ["general_outing", "weekend_outing"]) {
  const records = (pages[name] as Obj | undefined)?.records as Obj[] | undefined;
  records?.forEach((r, i) => (r.purpose = PURPOSES[i % PURPOSES.length]));
}

const history = (pages.grade_history as Obj | undefined)?.student as Obj | undefined;
if (history) Object.assign(history, { reg_no: FAKE.regno, name: FAKE.name });

// Placeholders VTOP fills empty fields with are not personal.
const BLANK = /^(nil|none|null|n\/a|na|not applicable|-+|0+)$/i;
const sweep = [...secrets.keys()].filter((s) => !BLANK.test(s)).sort((a, b) => b.length - a.length);
const scrub = (node: Json): Json => {
  if (typeof node === "string") return sweep.reduce((text, secret) => text.split(secret).join(secrets.get(secret) ?? "—"), node);
  if (Array.isArray(node)) return node.map(scrub);
  if (node && typeof node === "object") return Object.fromEntries(Object.entries(node).map(([k, v]) => [k, scrub(v)]));
  return node;
};
for (const key of Object.keys(pages)) {
  if (key === "profile" || key === "mentor") continue;
  pages[key] = scrub(pages[key]);
}

// Only the semesters with pages, so the picker offers nothing empty.
const recorded = new Set(Object.keys(pages).flatMap((key) => key.match(/@([^?]+)/)?.[1] ?? []));
const semesters = pages.semesters as { semesters: { id: string }[] } | undefined;
if (semesters) semesters.semesters = semesters.semesters.filter((s) => recorded.has(s.id));

const out = JSON.stringify({ schema: 1, pages });
const left = sweep.filter((secret) => out.includes(secret));
if (left.length > 0) throw new Error(`${left.length} personal value(s) still in the output`);
const path = decodeURIComponent(new URL("../guides/demo.json", import.meta.url).pathname);
writeFileSync(path, out);
console.log(`demo: ${Object.keys(pages).length} pages, ${sweep.length} values swept → guides/demo.json`);
