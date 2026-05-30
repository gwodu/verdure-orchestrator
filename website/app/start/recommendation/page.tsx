'use client';

import { useEffect, useMemo, useState, type FormEvent } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { getIntent, getTier } from '@/lib/session';
import type { Tier } from '@/lib/tiers';
import { classifyIntent } from '@/lib/intent';
import { recommend, type Recommendation, type InstallOption } from '@/lib/recommend';

export default function RecommendationScreen() {
  const router = useRouter();
  const [intent, setIntent] = useState<string | null>(null);
  const [tier, setTier] = useState<Tier | null>(null);

  useEffect(() => {
    const i = getIntent();
    const t = getTier();
    if (!i || !t) {
      router.replace('/start');
      return;
    }
    setIntent(i);
    setTier(t);
  }, [router]);

  const rec = useMemo<Recommendation | null>(() => {
    if (!intent || !tier) return null;
    return recommend(tier, classifyIntent(intent));
  }, [intent, tier]);

  if (!intent || !tier || !rec) return null;

  const notLocal = rec.feasibility === 'not-local-yet';

  return (
    <main className="bg-slate-800 text-white min-h-screen px-6 sm:px-12 py-16">
      <div className="max-w-3xl mx-auto">
        <p className="text-emerald-400 text-sm mb-3 tracking-widest uppercase">Step 4 of 4</p>

        {/* Verb-first headline: the benefit. The bold solution lands right below. */}
        <h1 className="text-3xl sm:text-4xl md:text-5xl font-bold mb-5 leading-tight">
          {rec.headline}
        </h1>
        <p className="text-lg text-slate-300 mb-8 leading-relaxed">{rec.body}</p>

        {/* The solution — bold and unmistakable, so no one has to hunt for the answer. */}
        {rec.model && (
          <div className="bg-slate-900 border-2 border-emerald-400/50 rounded-2xl p-6 sm:p-8 mb-8 shadow-lg">
            <div className="flex flex-wrap items-center gap-3 mb-2">
              <p className="text-emerald-400 text-xs sm:text-sm uppercase tracking-widest font-bold">
                Use this
              </p>
              {rec.feasibility === 'works-but-slow' && (
                <span className="bg-amber-400/15 text-amber-300 text-xs px-2.5 py-1 rounded-full font-medium">
                  Works, but slow on your device
                </span>
              )}
            </div>
            {rec.model.infoUrl ? (
              <a
                href={rec.model.infoUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-block text-4xl sm:text-5xl font-bold text-emerald-400 hover:text-emerald-300 underline-offset-8 hover:underline transition-colors"
              >
                {rec.model.name}
              </a>
            ) : (
              <span className="text-4xl sm:text-5xl font-bold text-emerald-400">{rec.model.name}</span>
            )}
            {rec.model.note && (
              <p className="text-slate-300 text-base sm:text-lg mt-3">{rec.model.note}</p>
            )}
          </div>
        )}

        {/* How to get the recommended tool — directly under the solution, no scrolling past a "coming soon". */}
        {rec.installOptions.length > 0 && (
          <div className="mb-10">
            <h2 className="text-2xl sm:text-3xl font-bold mb-5">How to get {rec.model?.name ?? 'it'}</h2>
            <div className="space-y-4">
              {rec.installOptions.map((opt) => (
                <InstallCard key={opt.name} option={opt} />
              ))}
            </div>
          </div>
        )}

        {/* Alternatives */}
        {rec.alternatives.length > 0 && (
          <div className="mb-10">
            <h3 className="text-lg font-semibold mb-3 text-slate-300">Other options</h3>
            <div className="space-y-2">
              {rec.alternatives.map((alt) => (
                <div key={alt.name} className="bg-slate-900/50 border border-slate-700 rounded-lg p-4">
                  <p className="font-semibold text-slate-200">{alt.name}</p>
                  <p className="text-slate-400 text-sm">{alt.reason}</p>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Verdure CTA — secondary to the answer above: the easier future, not the buried present. */}
        <h2 className="text-2xl sm:text-3xl font-bold mb-6">
          {notLocal ? 'In the meantime' : 'Want it even easier?'}
        </h2>
        <VerdureWaitlist intentText={intent} modelName={rec.model?.name ?? ''} tier={tier} notLocal={notLocal} />

        <div className="mt-12 text-center">
          <Link href="/start" className="text-slate-400 hover:text-white text-sm">
            ← Start over with different choices
          </Link>
        </div>
      </div>
    </main>
  );
}

/** Verdure waitlist — the most prominent CTA. */
function VerdureWaitlist({
  intentText,
  modelName,
  tier,
  notLocal,
}: {
  intentText: string;
  modelName: string;
  tier: Tier;
  notLocal: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [firstName, setFirstName] = useState('');
  const [email, setEmail] = useState('');
  const [status, setStatus] = useState<'idle' | 'submitting' | 'success' | 'error'>('idle');
  const [errorMsg, setErrorMsg] = useState<string>('');

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (status === 'submitting') return;
    setStatus('submitting');
    setErrorMsg('');
    try {
      const res = await fetch('/api/waitlist', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ firstName, email, intentText, modelName, tier }),
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

  // Copy adapts to context: for "not-local-yet" the waitlist is the main thing
  // on offer, framed as "we'll tell you when Verdure can do this for you."
  const blurb = notLocal
    ? 'We’ll do all of it for you — as easy as installing an app — and email you the moment your device can do this. Nothing technical to figure out.'
    : 'We’ll do all of it for you. No setup, no settings, nothing technical — as easy as installing any app. Join the waitlist and we’ll email you the moment it’s ready.';

  return (
    <div className="bg-emerald-400 text-slate-900 rounded-xl p-6 shadow-lg">
      <div className="flex flex-wrap items-start justify-between gap-3 mb-3">
        <div>
          <p className="text-xs uppercase tracking-wider font-bold opacity-70">Easiest</p>
          <h3 className="text-2xl font-bold">Let Verdure do it for you</h3>
        </div>
        <span className="bg-slate-900 text-emerald-400 text-xs px-3 py-1 rounded-full font-semibold">
          Coming soon
        </span>
      </div>
      <p className="text-slate-900/80 mb-4">{blurb}</p>

      {status === 'success' ? (
        <div className="bg-slate-900/10 border border-slate-900/20 rounded-lg p-4">
          <p className="font-semibold">You&rsquo;re on the list.</p>
          <p className="text-sm text-slate-900/80 mt-1">
            We&rsquo;ll email {email} when Verdure is ready. Thanks for telling us what you want to use it for — that&rsquo;s how we build the right thing first.
          </p>
        </div>
      ) : !open ? (
        <button
          onClick={() => setOpen(true)}
          className="bg-slate-900 text-emerald-400 px-6 py-3 rounded-full font-semibold hover:bg-slate-800 transition"
        >
          Join the waitlist
        </button>
      ) : (
        <form onSubmit={submit} className="space-y-3">
          <div className="grid sm:grid-cols-2 gap-3">
            <input
              type="text"
              required
              autoFocus
              value={firstName}
              onChange={(e) => setFirstName(e.target.value)}
              placeholder="First name"
              className="w-full p-3 bg-slate-900 text-white placeholder-slate-500 border border-slate-700 rounded-lg focus:border-emerald-300 focus:outline-none"
            />
            <input
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="you@example.com"
              className="w-full p-3 bg-slate-900 text-white placeholder-slate-500 border border-slate-700 rounded-lg focus:border-emerald-300 focus:outline-none"
            />
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <button
              type="submit"
              disabled={status === 'submitting'}
              className="bg-slate-900 text-emerald-400 px-6 py-3 rounded-full font-semibold hover:bg-slate-800 transition disabled:opacity-50"
            >
              {status === 'submitting' ? 'Joining…' : 'Join the waitlist'}
            </button>
            <button
              type="button"
              onClick={() => setOpen(false)}
              className="text-slate-900/70 hover:text-slate-900 text-sm"
            >
              Cancel
            </button>
          </div>
          {status === 'error' && (
            <p className="text-sm text-red-900 bg-red-200/60 border border-red-900/30 rounded-lg p-3">
              {errorMsg || 'Something went wrong. Please try again.'}
            </p>
          )}
          <p className="text-xs text-slate-900/70">
            We save your name, email, and what you typed so we can tell you when Verdure is ready and build the things people actually want.
          </p>
        </form>
      )}
    </div>
  );
}

/** A single modality-aware install path. Renders a copy-able command when one
 *  is present (the technical paths), a plain link otherwise (the friendly apps). */
function InstallCard({ option }: { option: InstallOption }) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    if (!option.command) return;
    try {
      await navigator.clipboard.writeText(option.command);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard unavailable — user can select & copy manually
    }
  };

  return (
    <div className="bg-slate-900 border border-slate-700 rounded-xl p-6">
      <div className="flex flex-wrap items-baseline justify-between gap-2 mb-1">
        <h3 className="text-xl font-bold">{option.name}</h3>
        {option.platforms && <span className="text-slate-500 text-xs">{option.platforms}</span>}
      </div>
      <p className="text-slate-300 mb-4">{option.detail}</p>

      {option.command && (
        <div className="mb-4">
          <div className="flex items-center gap-2 bg-slate-950 border border-slate-700 rounded-lg p-3 font-mono text-sm text-emerald-300">
            <code className="flex-1 overflow-x-auto whitespace-nowrap">{option.command}</code>
            <button
              onClick={copy}
              className="text-xs text-slate-400 hover:text-emerald-400 px-2 py-1 rounded transition shrink-0"
            >
              {copied ? 'Copied!' : 'Copy'}
            </button>
          </div>
        </div>
      )}

      <a
        href={option.url}
        target="_blank"
        rel="noopener noreferrer"
        className="inline-block border border-slate-600 text-white px-5 py-2.5 rounded-full text-sm font-semibold hover:border-emerald-400 transition"
      >
        Get {option.name} ↗
      </a>
    </div>
  );
}
