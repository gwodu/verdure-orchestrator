// Waitlist persistence — Google Sheets backend.
//
// One deep module hiding everything about how a waitlist signup gets durably
// stored: service-account auth, private-key parsing, column order, and the
// Sheets append call. Callers see one verb — appendWaitlistRow — plus a
// configuration probe so the API route can degrade gracefully before the
// Google Cloud setup is done.
//
// Setup (the Sheet, the service account, the env vars) lives in SETUP_SHEETS.md.
// Tracked as [OQ-10] in verdure-orchestrator/knowledge/open-questions.md.

import { auth, sheets } from '@googleapis/sheets';

/** A validated waitlist signup, as the API route assembles it. */
export type WaitlistSignup = {
  firstName: string;
  email: string;
  intentText: string;
  modelName: string;
  tier: string;
};

/** Resolved configuration for talking to the target Sheet. */
export type SheetsConfig = {
  clientEmail: string;
  privateKey: string;
  spreadsheetId: string;
  /** A1 range whose first column is where appended rows start. */
  range: string;
};

const SCOPES = ['https://www.googleapis.com/auth/spreadsheets'];

// The Sheet's columns, in order. Keep this in lockstep with the header row
// documented in SETUP_SHEETS.md — buildWaitlistRow emits values in this order.
export const WAITLIST_COLUMNS = [
  'Timestamp',
  'First name',
  'Email',
  'Intent',
  'Recommended model',
  'Tier',
] as const;

// Default tab + column span. The append API treats the range as a hint: it
// finds the table overlapping this range and appends after its last row, so
// "Sheet1!A:F" simply means "the six-column table on Sheet1".
const DEFAULT_RANGE = 'Sheet1!A:F';

/**
 * Service-account private keys arrive through env vars with their newlines
 * escaped as the two characters backslash-n. PEM parsing needs real newlines,
 * so unescape them. Already-real newlines (e.g. from a .env file with literal
 * line breaks) pass through untouched.
 */
export function normalizePrivateKey(raw: string): string {
  return raw.replace(/\\n/g, '\n');
}

/**
 * Read configuration from the environment. Returns null if any required
 * variable is missing — the signal that Sheets isn't wired up yet, which the
 * caller treats as "log and move on" rather than an error.
 */
export function loadSheetsConfig(
  env: Record<string, string | undefined> = process.env,
): SheetsConfig | null {
  const clientEmail = env.GOOGLE_SHEETS_CLIENT_EMAIL?.trim();
  const rawKey = env.GOOGLE_SHEETS_PRIVATE_KEY;
  const spreadsheetId = env.WAITLIST_SHEET_ID?.trim();
  const range = env.WAITLIST_SHEET_RANGE?.trim() || DEFAULT_RANGE;

  if (!clientEmail || !rawKey || !spreadsheetId) return null;

  return {
    clientEmail,
    privateKey: normalizePrivateKey(rawKey),
    spreadsheetId,
    range,
  };
}

/** Whether the Sheets backend is fully configured in this environment. */
export function isSheetsConfigured(
  env: Record<string, string | undefined> = process.env,
): boolean {
  return loadSheetsConfig(env) !== null;
}

/**
 * Serialize a signup into a row of cell values, ordered to match
 * WAITLIST_COLUMNS. Pure and deterministic given the timestamp, so the column
 * contract is unit-testable without touching the network.
 */
export function buildWaitlistRow(signup: WaitlistSignup, isoTimestamp: string): string[] {
  return [
    isoTimestamp,
    signup.firstName,
    signup.email,
    signup.intentText,
    signup.modelName,
    signup.tier,
  ];
}

/**
 * Append one signup as a new row in the configured Sheet.
 *
 * Throws if Sheets is configured but the append fails (network, auth, bad
 * Sheet ID) so the caller can surface a retryable error rather than silently
 * losing a real signup. The "not configured yet" case is the caller's to
 * detect via isSheetsConfigured — this function assumes it should write.
 */
export async function appendWaitlistRow(
  signup: WaitlistSignup,
  isoTimestamp: string,
): Promise<void> {
  const config = loadSheetsConfig();
  if (!config) {
    throw new Error('Google Sheets is not configured (see SETUP_SHEETS.md).');
  }

  const client = new auth.JWT({
    email: config.clientEmail,
    key: config.privateKey,
    scopes: SCOPES,
  });

  const service = sheets({ version: 'v4', auth: client });

  await service.spreadsheets.values.append({
    spreadsheetId: config.spreadsheetId,
    range: config.range,
    valueInputOption: 'RAW',
    insertDataOption: 'INSERT_ROWS',
    requestBody: { values: [buildWaitlistRow(signup, isoTimestamp)] },
  });
}
