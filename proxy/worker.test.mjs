/**
 * Translation tests for the Gemini path. No network: `fetch` is stubbed so we
 * can assert the exact request shape sent upstream and the exact response
 * shape returned to Baton.
 *
 * Run: node proxy/worker.test.mjs
 */
import assert from "node:assert/strict";
import worker from "./worker.js";

let captured = null;
const stubFetch = (response) => {
  globalThis.fetch = async (url, init) => {
    captured = { url, init, body: JSON.parse(init.body) };
    return new Response(JSON.stringify(response), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  };
};

const env = { PROVIDER: "gemini", GEMINI_API_KEY: "test-key" };

const batonRequest = (extra = {}) =>
  new Request("https://proxy/v1/messages", {
    method: "POST",
    headers: { "x-baton-device": "dev-1" },
    body: JSON.stringify({
      model: "claude-opus-5",
      max_tokens: 16000,
      system: "Extract a durable developer context.",
      messages: [{ role: "user", content: "<conversation>hi</conversation>" }],
      output_config: {
        effort: "medium",
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: { goal: { type: ["string", "null"] } },
            required: ["goal"],
            additionalProperties: false,
          },
        },
      },
      ...extra,
    }),
  });

const GEMINI_OK = {
  id: "int-1",
  status: "completed",
  model: "gemini-3.7-flash",
  steps: [
    { type: "model_response", content: [{ type: "text", text: '{"goal":"g"}' }] },
  ],
};

// --- request translation ---------------------------------------------------
stubFetch(GEMINI_OK);
await worker.fetch(batonRequest(), env);

assert.equal(
  captured.url,
  "https://generativelanguage.googleapis.com/v1beta/interactions",
);
assert.equal(captured.init.headers["x-goog-api-key"], "test-key");
assert.equal(captured.init.headers["Api-Revision"], "2026-05-20");
assert.equal(captured.body.system_instruction, "Extract a durable developer context.");
assert.equal(captured.body.input, "<conversation>hi</conversation>");
assert.equal(captured.body.generation_config.max_output_tokens, 16000);
assert.equal(captured.body.response_format.mime_type, "application/json");
assert.deepEqual(captured.body.response_format.schema.required, ["goal"]);
// The client-supplied model must be ignored — the proxy pins it.
assert.equal(captured.body.model, "gemini-3.7-flash");
console.log("ok  request translates Anthropic -> Gemini");

// --- response translation --------------------------------------------------
stubFetch(GEMINI_OK);
let res = await worker.fetch(batonRequest(), env);
let out = await res.json();
assert.equal(res.status, 200);
// Baton's text_of() looks for content[] blocks of type "text".
assert.equal(out.content[0].type, "text");
assert.equal(out.content[0].text, '{"goal":"g"}');
assert.equal(out.stop_reason, "end_turn");
console.log("ok  response translates Gemini -> Anthropic shape");

// --- multiple text blocks are joined --------------------------------------
stubFetch({
  status: "completed",
  steps: [
    { content: [{ type: "text", text: '{"goal":' }] },
    { content: [{ type: "text", text: '"g"}' }] },
  ],
});
out = await (await worker.fetch(batonRequest(), env)).json();
assert.equal(out.content[0].text, '{"goal":"g"}');
console.log("ok  split text blocks are concatenated");

// --- blocked responses surface as a refusal, not empty text ----------------
stubFetch({ status: "blocked", steps: [{ content: [{ type: "text", text: "" }] }] });
out = await (await worker.fetch(batonRequest(), env)).json();
assert.equal(out.stop_reason, "refusal");
console.log("ok  blocked -> stop_reason refusal");

// --- no text at all is an error, not a silent empty context ----------------
stubFetch({ status: "completed", steps: [] });
res = await worker.fetch(batonRequest(), env);
assert.equal(res.status, 502);
assert.match((await res.json()).error.message, /no text/);
console.log("ok  empty output is an error");

// --- routing / guards ------------------------------------------------------
res = await worker.fetch(
  new Request("https://proxy/v1/messages", { method: "GET" }),
  env,
);
assert.equal(res.status, 405);

res = await worker.fetch(
  new Request("https://proxy/other", { method: "POST", body: "{}" }),
  env,
);
assert.equal(res.status, 404);

res = await worker.fetch(
  new Request("https://proxy/v1/messages", {
    method: "POST",
    body: "x".repeat(1_000_001),
  }),
  env,
);
assert.equal(res.status, 413);
console.log("ok  method, path and size guards");

// --- rate limiting ---------------------------------------------------------
const store = new Map();
const kvEnv = {
  ...env,
  RATE_LIMIT: {
    get: async (k) => store.get(k) ?? null,
    put: async (k, v) => void store.set(k, v),
  },
};
stubFetch(GEMINI_OK);
for (let i = 0; i < 100; i++) await worker.fetch(batonRequest(), kvEnv);
res = await worker.fetch(batonRequest(), kvEnv);
assert.equal(res.status, 429, "101st request must be rejected");
console.log("ok  daily cap enforced at 100/device");

console.log("\nall proxy tests passed");
