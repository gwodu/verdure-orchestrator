// Waitlist capture endpoint.
//
// STUB IMPLEMENTATION (commit 2 of the website-build chapter).
// The next commit will replace the console.log below with an append to
// the Google Sheet configured via env vars (see [OQ-10] in
// verdure-orchestrator/knowledge/open-questions.md). For now the route
// logs to the server console and returns success so the UI flow is
// fully exercisable end-to-end. We are not in production yet — no
// real data is being captured or lost.

import { NextResponse, type NextRequest } from 'next/server';

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

  // TODO(OQ-10): replace with Google Sheets append via service account.
  console.log('[waitlist] new signup', {
    timestamp: new Date().toISOString(),
    firstName,
    email,
    intentText,
    modelName,
    tier,
  });

  return NextResponse.json({ ok: true });
}
