// Waitlist capture endpoint.
//
// Validates the signup, then hands persistence to lib/sheets.ts. Two modes:
//   - Sheets configured (env vars set, see SETUP_SHEETS.md): append a row;
//     surface a retryable error if that fails so no real signup is lost.
//   - Not configured yet: log the signup server-side and return success so the
//     four-screen flow stays fully exercisable before the Google Cloud setup
//     is done. Tracked as [OQ-10].

import { NextResponse, type NextRequest } from 'next/server';
import {
  appendWaitlistRow,
  isSheetsConfigured,
  type WaitlistSignup,
} from '@/lib/sheets';

type WaitlistPayload = {
  firstName?: unknown;
  email?: unknown;
  intentText?: unknown;
  modelName?: unknown;
  tier?: unknown;
};

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export async function POST(request: NextRequest) {
  let payload: WaitlistPayload;
  try {
    payload = await request.json();
  } catch {
    return NextResponse.json({ error: 'Body must be JSON' }, { status: 400 });
  }

  const firstName = typeof payload.firstName === 'string' ? payload.firstName.trim() : '';
  const email = typeof payload.email === 'string' ? payload.email.trim() : '';
  const intentText = typeof payload.intentText === 'string' ? payload.intentText : '';
  const modelName = typeof payload.modelName === 'string' ? payload.modelName : '';
  const tier = typeof payload.tier === 'string' ? payload.tier : '';

  if (!firstName || !email) {
    return NextResponse.json({ error: 'firstName and email are required' }, { status: 400 });
  }
  if (!EMAIL_RE.test(email)) {
    return NextResponse.json({ error: 'Please enter a valid email address' }, { status: 400 });
  }

  const signup: WaitlistSignup = { firstName, email, intentText, modelName, tier };
  const timestamp = new Date().toISOString();

  if (!isSheetsConfigured()) {
    // No backend wired up yet — don't lose the signup silently; log it and let
    // the UI flow proceed. Replace this path by setting the env vars.
    console.warn('[waitlist] Google Sheets not configured — logging signup only', {
      timestamp,
      ...signup,
    });
    return NextResponse.json({ ok: true });
  }

  try {
    await appendWaitlistRow(signup, timestamp);
  } catch (err) {
    console.error('[waitlist] failed to append to Google Sheet', err);
    return NextResponse.json(
      { error: 'We could not save your signup just now. Please try again.' },
      { status: 502 },
    );
  }

  return NextResponse.json({ ok: true });
}
