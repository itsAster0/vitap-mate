import { deflateSync, strToU8 } from "fflate";
import { describe, expect, it } from "vitest";
import { jsonBody } from "./common";

const post = (body: BodyInit, headers: Record<string, string> = {}) =>
  new Request("https://x/v1/data/refresh", { method: "POST", body, headers });

describe("jsonBody", () => {
  it("reads a plain JSON body", async () => {
    expect(await jsonBody(post('{"a":1}'))).toEqual({ a: 1 });
  });

  it("reads a deflate-raw body the phone compressed", async () => {
    const body = deflateSync(strToU8('{"have":{"k":"h"}}'));
    expect(await jsonBody(post(body, { "x-vtop-encoding": "deflate-raw" }))).toEqual({ have: { k: "h" } });
  });

  it("refuses a compressed body that expands past the limit", async () => {
    const huge = `{"a":"${"x".repeat(2_000_000)}"}`;
    const body = deflateSync(strToU8(huge));
    expect(await jsonBody(post(body, { "x-vtop-encoding": "deflate-raw" }))).toBeNull();
  });

  it("refuses a body that is not deflate-raw but says it is", async () => {
    expect(await jsonBody(post('{"a":1}', { "x-vtop-encoding": "deflate-raw" }))).toBeNull();
  });
});
