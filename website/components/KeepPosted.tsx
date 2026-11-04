'use client';

import { useState, type FormEvent } from 'react';

// One form: "send me a test build" on the Try page. It lands in the waitlist
// sheet (lib/sheets.ts) through /api/waitlist, with `intentText` saying which
// form it came from, so nothing new had to be built on the server to make the
// beta reachable from a phone.
export default function KeepPosted({
  intent,
  title,
  blurb,
  button,
  platforms,
}: {
  intent: string;
  title: string;
  blurb: string;
  button: string;
  /** When given, a platform choice is asked for and sent as `tier`. */
  platforms?: { id: string; label: string }[];
}) {
  const [firstName, setFirstName] = useState('');
  const [email, setEmail] = useState('');
  const [platform, setPlatform] = useState(platforms?.[0]?.id ?? '');
  const [status, setStatus] = useState<'idle' | 'submitting' | 'success' | 'error'>('idle');
  const [errorMsg, setErrorMsg] = useState('');

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (status === 'submitting') return;
    setStatus('submitting');
    setErrorMsg('');
    try {
      const res = await fetch('/api/waitlist', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ firstName, email, intentText: intent, modelName: '', tier: platform }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body?.error ?? 'Something went wrong');
      }
      setStatus('success');
    } catch (err) {
      setStatus('error');
      setErrorMsg(err instanceof Error ? err.message : 'Something went wrong');
    }
  };

  const field =
    'w-full p-3 rounded-lg bg-white text-ink placeholder-ink-soft border border-line focus:border-green focus:outline-none';

  return (
    <div className="card p-6 sm:p-8">
      <h3 className="text-xl sm:text-2xl font-semibold tracking-tight mb-2">{title}</h3>
      <p className="text-ink-2 mb-5">{blurb}</p>
      {status === 'success' ? (
        <p className="text-green font-semibold">
          Got it. We&rsquo;ll write to {email}. Nothing else happens with your address.
        </p>
      ) : (
        <form onSubmit={submit} className="space-y-3">
          <div className={`grid gap-3 ${platforms ? 'sm:grid-cols-3' : 'sm:grid-cols-2'}`}>
            <input type="text" required value={firstName} onChange={(e) => setFirstName(e.target.value)} placeholder="First name" aria-label="First name" className={field} />
            <input type="email" required value={email} onChange={(e) => setEmail(e.target.value)} placeholder="you@example.com" aria-label="Email" className={field} />
            {platforms && (
              <select value={platform} onChange={(e) => setPlatform(e.target.value)} aria-label="Platform" className={field}>
                {platforms.map((p) => (
                  <option key={p.id} value={p.id}>{p.label}</option>
                ))}
              </select>
            )}
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <button type="submit" disabled={status === 'submitting'} className="btn-primary">
              {status === 'submitting' ? 'Sending…' : button}
            </button>
            {status === 'error' && <p className="text-red-700 text-sm">{errorMsg}</p>}
          </div>
        </form>
      )}
    </div>
  );
}
