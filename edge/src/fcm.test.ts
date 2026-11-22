import { describe, expect, it } from "vitest";

import { type ServiceAccount, message, pemToDer, unsignedJwt } from "./fcm";
import { fromBase64Url } from "./shared/base64";

const account: ServiceAccount = {
  client_email: "push@p.iam.gserviceaccount.com",
  private_key: "-----BEGIN PRIVATE KEY-----\nAAEC\nAw==\n-----END PRIVATE KEY-----\n",
  project_id: "p",
  token_uri: "https://oauth2.googleapis.com/token",
};

function decode(part: string) {
  return JSON.parse(new TextDecoder().decode(fromBase64Url(part)!));
}

describe("fcm", () => {
  it("asserts the right claims", () => {
    const [header, claims] = unsignedJwt(account, 100).split(".");
    expect(decode(header).alg).toBe("RS256");
    expect(decode(claims)).toMatchObject({
      iss: "push@p.iam.gserviceaccount.com",
      aud: "https://oauth2.googleapis.com/token",
      scope: "https://www.googleapis.com/auth/firebase.messaging",
      exp: 3700,
    });
  });

  it("decodes the PEM body", () => {
    expect(pemToDer(account.private_key)).toEqual(new Uint8Array([0, 1, 2, 3]));
  });

  it("sends data values as strings", () => {
    const body = message("tok", { type: "mail_otp", seq: 7 });
    expect(body.message.data).toEqual({ type: "mail_otp", seq: "7" });
    expect(body.message.android.priority).toBe("HIGH");
  });
});
