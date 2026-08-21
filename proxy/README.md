# Baton API proxy

Holds the provider API key so the desktop app never ships one, and decides
**which** provider Baton actually talks to.

**Why it exists:** anything in a distributed binary is public. `strings` on the
app, or a debugging proxy watching the wire, recovers an embedded key in
seconds — and the resulting bill has no ceiling.

## Provider switching lives here, not in the app

Baton always speaks the **Anthropic Messages** wire format. This Worker
translates to whatever `PROVIDER` names.

That split is deliberate. Baton is a desktop app: changing providers in the
client would mean rebuilding and redistributing binaries to every user. Here
it is a one-line deploy — and you can switch, or fall back, without anyone
updating anything.

| `PROVIDER` | Upstream | Default model |
|---|---|---|
| `gemini` (default) | `generativelanguage.googleapis.com/v1beta/interactions` | `gemini-3.7-flash` |
| `anthropic` | `api.anthropic.com/v1/messages` | `claude-opus-5` |

`MODEL` overrides the default. For higher extraction quality on Gemini, try
`gemini-3.1-pro-preview` — it is a preview model, so pin deliberately.

## Deploy

```bash
npm create cloudflare@latest baton-proxy -- --type=hello-world
cd baton-proxy
# replace src/index.js with worker.js from this directory

npx wrangler secret put GEMINI_API_KEY         # or ANTHROPIC_API_KEY
npx wrangler kv namespace create RATE_LIMIT    # then add the binding below
npx wrangler deploy
```

`wrangler.toml`:

```toml
[vars]
PROVIDER = "gemini"
# MODEL = "gemini-3.1-pro-preview"   # optional override

[[kv_namespaces]]
binding = "RATE_LIMIT"
id = "<id printed by the create command>"
```

The proxy runs without KV — it just stops enforcing the daily cap.

## Point the app at it

```bash
BATON_API_BASE=https://your-worker.workers.dev pnpm tauri build
```

The placeholder default in `src-tauri/src/ai.rs` is `https://api.baton.app`,
which **does not exist**. Change that constant once you have a real domain.

## Tests

```bash
node proxy/worker.test.mjs
```

Stubs `fetch` and asserts the exact request sent upstream and the exact
response handed back to Baton — including split text blocks, blocked responses
mapping to `stop_reason: "refusal"`, empty output failing loudly rather than
saving an empty context, and the rate limit.

## What it enforces

| Control | Why |
|---|---|
| `POST /v1/messages` only | nothing else needs to be reachable |
| Model pinned server-side | otherwise callers pick the model *you* pay for |
| 1MB body cap | rejects oversized pastes before they cost anything |
| 100 requests/device/day | bounds worst-case spend |

The device id is client-generated and spoofable. It is friction against casual
abuse, not authentication. If the endpoint leaks publicly, move to signed
tokens or accounts.

## Cost

Per extraction — a long conversation in, a structured context out — roughly:

| Model | Typical | Full-session dump |
|---|---|---|
| `gemini-3.7-flash` | fractions of a cent | a few cents |
| `claude-opus-5` | ~$0.12 | ~$0.50 |

Gemini Flash is dramatically cheaper for this workload, which is one reason it
is the default. Confirm current rates at
<https://ai.google.dev/pricing> before budgeting.

## Privacy

Every pasted conversation transits this Worker. Those conversations routinely
contain API keys, internal URLs, and unreleased work. **Do not log request
bodies** — the code deliberately does not.

PRD §14 currently promises content goes only to the LLM provider. That wording
needs updating: it now passes through your infrastructure first.
