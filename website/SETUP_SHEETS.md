# Waitlist → Google Sheets setup

The `/api/waitlist` endpoint appends each signup as a row in a Google Sheet.
Until the environment variables below are set, the endpoint logs signups to the
server console and returns success (so the flow stays testable) — it does **not**
save anything. Do this setup before launch.

You need three things: a Sheet, a service account that can write to it, and
three environment variables wiring them together.

## 1. Create the Sheet

1. Make a new Google Sheet (e.g. "Verdure waitlist").
2. In the first row of the first tab, add these headers, left to right:

   | Timestamp | First name | Email | Intent | Recommended model | Tier |
   |-----------|------------|-------|--------|-------------------|------|

   The header text is for you — the code appends by column position, not name —
   but keep the **order** identical to the above.
3. Copy the **Sheet ID** from the URL. In
   `https://docs.google.com/spreadsheets/d/`**`1AbC...xyz`**`/edit`, the bold
   part is the ID.

> If your tab isn't named `Sheet1`, or you want a different column span, set
> `WAITLIST_SHEET_RANGE` (see below). Otherwise you can skip it.

## 2. Create a service account

A service account is a robot Google identity the server uses to write rows.

1. Go to the [Google Cloud Console](https://console.cloud.google.com/) and
   create a project (or pick an existing one).
2. Enable the **Google Sheets API**: APIs & Services → Library → search
   "Google Sheets API" → Enable.
3. Create the account: APIs & Services → Credentials → Create credentials →
   Service account. Give it a name like `verdure-waitlist`. No roles needed.
4. Open the new service account → **Keys** tab → Add key → Create new key →
   **JSON**. A `.json` file downloads. Keep it secret — it's a credential.

The JSON contains a `client_email` (looks like
`verdure-waitlist@your-project.iam.gserviceaccount.com`) and a `private_key`
(a long `-----BEGIN PRIVATE KEY----- ...` block). You'll need both.

## 3. Share the Sheet with the service account

The robot can't see your Sheet until you invite it.

1. Open the Sheet → **Share**.
2. Paste the service account's `client_email`.
3. Give it **Editor** access. Send.

## 4. Set the environment variables

Three variables (a fourth is optional). Locally, put them in `.env.local`
(git-ignored); on Vercel, add them under Project → Settings → Environment
Variables. See `.env.example` for the template.

| Variable | Value |
|----------|-------|
| `GOOGLE_SHEETS_CLIENT_EMAIL` | the `client_email` from the JSON |
| `GOOGLE_SHEETS_PRIVATE_KEY` | the `private_key` from the JSON (see note) |
| `WAITLIST_SHEET_ID` | the Sheet ID from step 1 |
| `WAITLIST_SHEET_RANGE` | *(optional)* default `Sheet1!A:F` — set only if your tab isn't `Sheet1` |

**Private key newlines.** The `private_key` is one long string with `\n`
escape sequences in it. Two ways to set it:

- **In `.env.local`** — wrap the whole value in double quotes and leave the
  `\n` sequences literal. The code converts them to real newlines:
  ```
  GOOGLE_SHEETS_PRIVATE_KEY="-----BEGIN PRIVATE KEY-----\nMIIEv...\n-----END PRIVATE KEY-----\n"
  ```
- **On Vercel** — paste the key as-is from the JSON (with the `\n` sequences).
  The same conversion runs server-side.

## 5. Verify

```bash
npm run dev
```

Walk the flow to the recommendation screen, join the waitlist, and confirm a
new row appears in the Sheet. If it doesn't, check the server console:

- `Google Sheets not configured` → a variable is missing or misspelled.
- `failed to append to Google Sheet` → the API isn't enabled, the Sheet isn't
  shared with the service account, or the Sheet ID is wrong. The error detail
  follows on the same log line.

## Notes

- The endpoint never silently drops a real signup: once configured, an append
  failure returns a retryable error to the user instead of a false success.
- No personal data beyond name, email, and what the user typed is stored. See
  `verdure-orchestrator/knowledge/06-privacy-principles.md`.
