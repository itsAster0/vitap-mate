// Catch-all Email Worker for the mail domain.
//
// vtop-otp-<id>@… : parse the mail in memory, seal the code to the phone's
// public key, hand only the sealed box to vtop-bridge. Nothing is stored
// and nothing about the content is logged.
// Anything else  : forwarded to FALLBACK_FORWARD (the domain's previous
// catch-all), or rejected when that is unset.

import PostalMime from "postal-mime";
import { mailboxId, payloadFor } from "./parse.js";
import { seal } from "./seal.js";

async function bridge(env, id, init = {}) {
  return fetch(`${env.BRIDGE_URL}/v1/mail-otp/relay/${id}`, {
    ...init,
    headers: {
      authorization: `Bearer ${env.RELAY_SECRET}`,
      "content-type": "application/json",
    },
  });
}

export default {
  async email(message, env, ctx) {
    const id = mailboxId(message.to);
    if (!id) {
      if (env.FALLBACK_FORWARD) {
        await message.forward(env.FALLBACK_FORWARD);
      } else {
        message.setReject("Unknown address");
      }
      return;
    }

    const keyResponse = await bridge(env, id);
    if (keyResponse.status === 404) {
      message.setReject("Unknown address");
      return;
    }
    if (!keyResponse.ok) {
      // Throwing makes the sending server retry later.
      throw new Error(`key lookup failed: ${keyResponse.status}`);
    }
    const { publicKey } = await keyResponse.json();

    const email = await PostalMime.parse(message.raw);
    const payload = payloadFor(email);
    if (!payload) {
      console.log(`mail-otp: dropped a mail for a mailbox`);
      return;
    }
    const sealed = await seal(publicKey, id, JSON.stringify(payload));
    const deliver = await bridge(env, id, {
      method: "POST",
      body: JSON.stringify({ sealed }),
    });
    if (!deliver.ok) throw new Error(`deliver failed: ${deliver.status}`);
    console.log(`mail-otp: relayed a ${payload.kind}`);
  },
};
