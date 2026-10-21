import { validateCookies, type CookieEditorCookie } from "./bridge";

export const VTOP_DOMAIN = "vtop.vitap.ac.in";

/** The parts of `chrome.cookies` used here, so tests can pass a fake. */
export type CookieApi = {
  getAll(details: { domain: string }): Promise<chrome.cookies.Cookie[]>;
  set(details: chrome.cookies.SetDetails): Promise<chrome.cookies.Cookie | null>;
  remove(details: {
    url: string;
    name: string;
    storeId?: string;
  }): Promise<chrome.cookies.CookieDetails | null>;
};

export function getCookieUrl(
  cookie: Pick<CookieEditorCookie, "domain" | "path" | "secure">,
) {
  const cleanDomain = cookie.domain.replace(/^\./, "");
  const path = cookie.path ?? "/";
  const protocol = cookie.secure === false ? "http" : "https";

  return `${protocol}://${cleanDomain}${path}`;
}

export function normalizeSameSite(value: CookieEditorCookie["sameSite"]) {
  switch (value) {
    case "lax":
    case "strict":
    case "no_restriction":
      return value;
    default:
      return "unspecified";
  }
}

export function chromeCookieToEditorCookie(cookie: chrome.cookies.Cookie): CookieEditorCookie {
  return {
    domain: cookie.domain,
    hostOnly: cookie.hostOnly,
    httpOnly: cookie.httpOnly,
    name: cookie.name,
    path: cookie.path,
    sameSite: cookie.sameSite,
    secure: cookie.secure,
    session: cookie.session,
    storeId: cookie.storeId,
    value: cookie.value,
    expirationDate: cookie.expirationDate,
  };
}

export async function clearVtopCookies(api: CookieApi, cookies: chrome.cookies.Cookie[]) {
  for (const cookie of cookies) {
    const cleanDomain = cookie.domain.replace(/^\./, "");
    const protocol = cookie.secure ? "https" : "http";

    const urls = [
      `${protocol}://${cleanDomain}${cookie.path}`,
      `${protocol}://${cleanDomain}/`,
      `https://${cleanDomain}${cookie.path}`,
      `https://${cleanDomain}/vtop`,
      `https://${cleanDomain}/`,
    ];

    for (const url of urls) {
      try {
        const removed = await api.remove({
          url,
          name: cookie.name,
          storeId: cookie.storeId,
        });

        if (removed) {
          break;
        }
      } catch (err) {
        console.warn("VITAP Mate: cookie removal failed", err);
      }
    }
  }
}

export async function setCookieFromCookieEditorFormat(api: CookieApi, cookie: CookieEditorCookie) {
  if (!cookie.name) throw new Error("Cookie missing name");
  if (!cookie.domain) throw new Error(`Cookie ${cookie.name} missing domain`);
  if (cookie.value === undefined || cookie.value === null) {
    throw new Error(`Cookie ${cookie.name} missing value`);
  }

  const details: chrome.cookies.SetDetails = {
    url: getCookieUrl(cookie),
    name: cookie.name,
    value: cookie.value,
    path: cookie.path ?? "/vtop",
    secure: cookie.secure ?? true,
    httpOnly: cookie.httpOnly ?? true,
    sameSite: normalizeSameSite(cookie.sameSite),
    storeId: cookie.storeId,
  };

  // Cookie-Editor hostOnly:true means do NOT pass domain.
  if (!cookie.hostOnly) {
    details.domain = cookie.domain;
  }

  // Session cookie means no expirationDate.
  if (!cookie.session && cookie.expirationDate) {
    details.expirationDate = cookie.expirationDate;
  }

  const result = await api.set(details);

  if (!result) {
    throw new Error(`Chrome failed to set cookie: ${cookie.name}`);
  }
}

async function restoreCookies(api: CookieApi, cookies: chrome.cookies.Cookie[]) {
  const current = await api.getAll({ domain: VTOP_DOMAIN });
  await clearVtopCookies(api, current);
  for (const cookie of cookies) {
    await setCookieFromCookieEditorFormat(api, chromeCookieToEditorCookie(cookie));
  }
}

/**
 * Replaces the browser's VTOP cookies with `cookies`. If any of them can't
 * be set, puts the previous ones back so the browser isn't left half
 * signed in.
 */
export async function installCookies(
  api: CookieApi,
  cookies: CookieEditorCookie[],
  report: (text: string) => Promise<void> | void = () => {},
) {
  validateCookies(cookies);

  await report("Clearing old cookies...");
  const previousCookies = await api.getAll({ domain: VTOP_DOMAIN });
  await clearVtopCookies(api, previousCookies);

  await report("Setting cookies...");
  try {
    for (const cookie of cookies) {
      // The sender's store id is Chrome's ("0"); Firefox names its stores
      // differently and rejects it. Without one, the browser uses its own.
      const { storeId: _, ...forThisBrowser } = cookie;
      await setCookieFromCookieEditorFormat(api, forThisBrowser);
    }
  } catch (err) {
    try {
      await restoreCookies(api, previousCookies);
    } catch (restoreError) {
      console.error("VITAP Mate: cookie rollback failed", restoreError);
    }
    throw err;
  }
}
