import { describe, expect, it } from "vitest";
import {
  getCookieUrl,
  installCookies,
  normalizeSameSite,
  type CookieApi,
} from "./cookies";

/** An in-memory cookie jar; `failOn` makes setting that cookie fail. */
function fakeJar(initial: Partial<chrome.cookies.Cookie>[], failOn?: string) {
  let jar = initial.map(
    (cookie) =>
      ({
        domain: "vtop.vitap.ac.in",
        path: "/vtop",
        secure: true,
        hostOnly: true,
        httpOnly: true,
        session: true,
        sameSite: "unspecified",
        storeId: "0",
        ...cookie,
      }) as chrome.cookies.Cookie,
  );
  const api: CookieApi = {
    async getAll() {
      return [...jar];
    },
    async set(details) {
      if (details.name === failOn) return null;
      jar = jar.filter((cookie) => cookie.name !== details.name);
      jar.push({
        domain: details.domain ?? "vtop.vitap.ac.in",
        name: details.name ?? "",
        value: details.value ?? "",
        path: details.path ?? "/",
        secure: details.secure ?? true,
        hostOnly: !details.domain,
        httpOnly: details.httpOnly ?? true,
        session: details.expirationDate === undefined,
        sameSite: details.sameSite ?? "unspecified",
        storeId: details.storeId ?? "0",
      });
      return jar[jar.length - 1];
    },
    async remove({ name }) {
      const before = jar.length;
      jar = jar.filter((cookie) => cookie.name !== name);
      return jar.length < before ? ({ name } as chrome.cookies.CookieDetails) : null;
    },
  };
  return { api, values: () => Object.fromEntries(jar.map((c) => [c.name, c.value])) };
}

const fresh = (name: string, value: string) => ({
  domain: "vtop.vitap.ac.in",
  name,
  value,
  path: "/vtop",
  hostOnly: true,
});

describe("cookies", () => {
  it("builds the cookie URL", () => {
    expect(getCookieUrl({ domain: ".vtop.vitap.ac.in", path: "/vtop" })).toBe(
      "https://vtop.vitap.ac.in/vtop",
    );
    expect(getCookieUrl({ domain: "vtop.vitap.ac.in", secure: false })).toBe(
      "http://vtop.vitap.ac.in/",
    );
  });

  it("maps sameSite", () => {
    expect(normalizeSameSite("lax")).toBe("lax");
    expect(normalizeSameSite(undefined)).toBe("unspecified");
  });

  it("replaces the old cookies", async () => {
    const jar = fakeJar([{ name: "JSESSIONID", value: "old" }, { name: "stale", value: "x" }]);
    await installCookies(jar.api, [fresh("JSESSIONID", "new")]);
    expect(jar.values()).toEqual({ JSESSIONID: "new" });
  });

  it("puts the old cookies back when one can't be set", async () => {
    const jar = fakeJar([{ name: "JSESSIONID", value: "old" }], "SERVERID");
    await expect(
      installCookies(jar.api, [fresh("JSESSIONID", "new"), fresh("SERVERID", "s")]),
    ).rejects.toThrow("SERVERID");
    expect(jar.values()).toEqual({ JSESSIONID: "old" });
  });

  it("rejects cookies for another site before touching the jar", async () => {
    const jar = fakeJar([{ name: "JSESSIONID", value: "old" }]);
    await expect(
      installCookies(jar.api, [{ domain: "example.com", name: "a", value: "b" }]),
    ).rejects.toThrow("unexpected domain");
    expect(jar.values()).toEqual({ JSESSIONID: "old" });
  });

  it("leaves the cookie store to the browser (Firefox has no store \"0\")", async () => {
    const jar = fakeJar([]);
    const set = jar.api.set;
    // As Firefox does: only its own store names are valid.
    jar.api.set = async (details) => {
      if (details.storeId !== undefined && details.storeId !== "firefox-default") {
        throw new Error(`Invalid cookie store id: "${details.storeId}"`);
      }
      return set(details);
    };
    await installCookies(jar.api, [{ ...fresh("JSESSIONID", "new"), storeId: "0" }]);
    expect(jar.values()).toEqual({ JSESSIONID: "new" });
  });
});
