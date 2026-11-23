// The authorize page an MCP client opens (edge/src/oauth.ts, unchanged): the
// client's name, a pairing code and its QR, and a script that polls
// `/authorize/poll`.

import { randomHex } from "./crypto";
import { qrSvg } from "./qr";

const REQUEST_TTL_SECS = 600;

function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

const STYLE = `
:root{color-scheme:dark;--bg:#0f172a;--card:#1e293b;--line:rgba(99,102,241,.4);--tint:rgba(99,102,241,.1);--text:#e2e8f0;--muted:#94a3b8;--code:#e0e7ff}
*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;background:var(--bg);color:var(--text);font:15px/1.5 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;padding:16px}
main{width:100%;max-width:380px;background:var(--card);border-radius:16px;padding:24px;text-align:center}
h1{font-size:19px;margin:0 0 4px}p{margin:8px 0;color:var(--muted);font-size:13px}
.pair{border:1px solid var(--line);background:var(--tint);border-radius:12px;padding:12px;margin:16px 0}
.pair svg{width:144px;height:144px;border-radius:6px;display:block;margin:4px auto}
.code{font:600 20px ui-monospace,SFMono-Regular,Menlo,monospace;letter-spacing:.15em;color:var(--code);margin:6px 0 0}
ol{text-align:left;color:var(--muted);font-size:13px;padding-left:20px;margin:8px 0}b{color:var(--text)}`;

export function page(title: string, body: string, status = 200, script = ""): Response {
  const nonce = randomHex(16);
  const html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="referrer" content="no-referrer"><title>${escapeHtml(title)} · vitapmate</title><style nonce="${nonce}">${STYLE}</style></head><body><main><h1>${escapeHtml(title)}</h1>${body}</main>${script ? `<script nonce="${nonce}">${script}</script>` : ""}</body></html>`;
  return new Response(html, {
    status,
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-store",
      "content-security-policy": `default-src 'none'; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}'; img-src data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'`,
      "x-frame-options": "DENY",
      "referrer-policy": "no-referrer",
    },
  });
}

export function pairingPage(client: { name: string }, host: string, opened: { pairCode: string; poll: string }): Response {
  const shown = `${opened.pairCode.slice(0, 5)}-${opened.pairCode.slice(5)}`;
  const body = `<p><b>${escapeHtml(client.name)}</b> (${escapeHtml(host)}) wants to use your VTOP data through vitapmate.</p>
<div class="pair">${qrSvg(shown) ?? ""}<p class="code" aria-label="Pairing code">${shown}</p></div>
<ol><li>Open the vitapmate app</li><li>Connected apps → <b>Pair a browser</b></li><li>Scan the QR code or enter the code shown here</li></ol>
<p id="status">Waiting for the app… The code works for ${Math.round(REQUEST_TTL_SECS / 60)} minutes.</p>`;
  // The poll id is 64 hex characters, safe to put in the script as is.
  const script = `(function(){var s=document.getElementById("status");function tick(){fetch("/authorize/poll",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify({poll:"${opened.poll}"})}).then(function(r){return r.json()}).then(function(j){if(j.status==="ready"){s.textContent="Approved. Going back to the app…";location.replace(j.redirect)}else if(j.status==="pending"){setTimeout(tick,j.pollAfterMs||2000)}else{s.textContent="The code has expired. Start connecting again from the app."}}).catch(function(){setTimeout(tick,4000)})}setTimeout(tick,2000)})();`;
  return page(`Connect ${client.name}`, body, 200, script);
}
