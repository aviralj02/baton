/**
 * Baton API proxy — Cloudflare Worker.
 *
 * Holds the provider API key so the desktop app never ships one. An embedded
 * key is a public key: `strings` on the binary or a debugging proxy on the wire
 * recovers it in seconds, and the bill has no ceiling.
 *
 * ---------------------------------------------------------------------------
 * PROVIDER SWITCHING LIVES HERE, NOT IN THE APP.
 *
 * Baton always speaks the Anthropic Messages wire format. This Worker
 * translates to whichever provider `PROVIDER` names. That matters because
 * Baton is a desktop app: changing providers client-side would mean rebuilding
 * and redistributing binaries to every user, while changing them here is a
 * deploy. It also means you can switch providers, or fall back, without anyone
 * updating anything.
 * ---------------------------------------------------------------------------
 *
 * Deploy:
 *   npm create cloudflare@latest baton-proxy -- --type=hello-world
 *   # replace src/index.js with this file, then:
 *   npx wrangler secret put GEMINI_API_KEY        # and/or ANTHROPIC_API_KEY
 *   npx wrangler deploy
 *
 * Configure (wrangler.toml [vars]):
 *   PROVIDER = "gemini"            # or "anthropic"
 *   MODEL    = "gemini-3.7-flash"  # optional; overrides the default below
 */

/** Per-device daily request cap. The only thing between you and a large bill. */
const DAILY_LIMIT = 100;

/** Reject oversized bodies before they cost anything. ~1M chars ≈ 250k tokens. */
const MAX_BODY_BYTES = 1_000_000;

const DEFAULTS = {
  anthropic: "claude-opus-5",
  // Current stable flagship Flash — strong at extraction, and extraction is a
  // compression task rather than a hard reasoning one. Swap to
  // "gemini-3.1-pro-preview" via the MODEL var if quality needs it.
  gemini: "gemini-3.7-flash",
};

export default {
  async fetch(request, env) {
    if (request.method !== "POST") return err(405, "POST only");
    if (new URL(request.url).pathname !== "/v1/messages") return err(404, "not found");

    const raw = await request.text();
    if (raw.length > MAX_BODY_BYTES) return err(413, "conversation too large");

    let req;
    try {
      req = JSON.parse(raw);
    } catch {
      return err(400, "invalid JSON");
    }

    // Rate limit per install. The device id is client-generated and trivially
    // spoofable — friction against casual abuse, not authentication.
    const device = request.headers.get("x-baton-device") || "unknown";
    if (env.RATE_LIMIT) {
      const key = `${device}:${new Date().toISOString().slice(0, 10)}`;
      const used = Number((await env.RATE_LIMIT.get(key)) || 0);
      if (used >= DAILY_LIMIT) return err(429, "daily limit reached");
      // Best-effort: concurrent requests can race. Acceptable for a cap whose
      // job is bounding cost, not exact accounting.
      await env.RATE_LIMIT.put(key, String(used + 1), { expirationTtl: 172800 });
    }

    const provider = (env.PROVIDER || "gemini").toLowerCase();
    // The model is pinned server-side. Whatever the client asks for is ignored:
    // otherwise anyone who finds the endpoint picks the model you pay for.
    const model = env.MODEL || DEFAULTS[provider];
    if (!model) return err(500, `unknown provider "${provider}"`);

    try {
      return provider === "anthropic"
        ? await callAnthropic(req, model, env, request)
        : await callGemini(req, model, env);
    } catch (e) {
      return err(502, `upstream failure: ${e.message}`);
    }
  },
};

// --------------------------------------------------------------- Anthropic
// Near-passthrough: Baton already speaks this format.

async function callAnthropic(req, model, env, request) {
  const upstream = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-api-key": env.ANTHROPIC_API_KEY,
      "anthropic-version": "2023-06-01",
      ...(request.headers.get("anthropic-beta")
        ? { "anthropic-beta": request.headers.get("anthropic-beta") }
        : {}),
    },
    body: JSON.stringify({ ...req, model }),
  });

  // Never log the body — it is the user's conversation (PRD §9).
  return new Response(upstream.body, {
    status: upstream.status,
    headers: { "content-type": "application/json" },
  });
}

// ------------------------------------------------------------------ Gemini
// Translate Anthropic Messages -> Gemini Interactions, and the response back.

async function callGemini(req, model, env) {
  const body = {
    model,
    // Baton sends exactly one user message; there is no multi-turn chat here.
    input: req.messages?.map((m) => m.content).join("\n\n") ?? "",
    generation_config: { max_output_tokens: req.max_tokens ?? 16000 },
  };

  if (req.system) body.system_instruction = req.system;

  // Anthropic: output_config.format.schema -> Gemini: response_format.schema.
  // The JSON Schema itself carries over unchanged, nullable unions included.
  const schema = req.output_config?.format?.schema;
  if (schema) {
    body.response_format = {
      type: "text",
      mime_type: "application/json",
      schema,
    };
  }

  const upstream = await fetch(
    "https://generativelanguage.googleapis.com/v1beta/interactions",
    {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-goog-api-key": env.GEMINI_API_KEY,
        "Api-Revision": "2026-05-20",
      },
      body: JSON.stringify(body),
    },
  );

  const text = await upstream.text();
  if (!upstream.ok) {
    let message = upstream.statusText;
    try {
      message = JSON.parse(text)?.error?.message || message;
    } catch {
      /* keep the status text */
    }
    return err(upstream.status, message);
  }

  const data = JSON.parse(text);

  // Generated text lives in steps[].content[] blocks of type "text"; the
  // official helper concatenates the trailing text blocks, so we do the same.
  const parts = [];
  for (const step of data.steps ?? []) {
    for (const block of step.content ?? []) {
      if (block.type === "text" && typeof block.text === "string") {
        parts.push(block.text);
      }
    }
  }

  if (parts.length === 0) {
    return err(502, "provider returned no text");
  }

  // Re-shape into what Baton's parser expects: a content array of text blocks
  // plus a stop_reason it can check for refusals.
  return json(
    {
      content: [{ type: "text", text: parts.join("") }],
      stop_reason: data.status === "blocked" ? "refusal" : "end_turn",
      model: data.model ?? model,
    },
    200,
  );
}

// ------------------------------------------------------------------ helpers

function json(obj, status) {
  return new Response(JSON.stringify(obj), {
    status,
    headers: { "content-type": "application/json" },
  });
}

/** Error shape Baton already parses: `error.message`. */
function err(status, message) {
  return json({ error: { message } }, status);
}
