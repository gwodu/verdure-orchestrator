'use client';

import { useEffect, useMemo, useState, type FormEvent } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { getIntent, getTier } from '@/lib/session';
import type { Tier } from '@/lib/tiers';
import { classifyIntent } from '@/lib/intent';
import { recommend, type Recommendation } from '@/lib/recommend';

// Map a recommended model name to its Ollama tag. Kept local for v1; if a
// second install backend (Jan, LM Studio deep-link) needs a similar mapping,
// lift this into lib/recommend.ts as fields on the Recommendation type.
const OLLAMA_TAGS: Record<string, string> = {
  'Gemma 3 1B': 'gemma3:1b',
  'Gemma 3 4B': 'gemma3:4b',
  'Llama 3.2 3B': 'llama3.2:3b',
  'Llama 3.1 8B': 'llama3.1:8b',
  'Phi-4-mini': 'phi4-mini',
  'Qwen 2.5 1.5B': 'qwen2.5:1.5b',
  'Qwen 2.5 3B': 'qwen2.5:3b',
  'Mistral Small': 'mistral-small',
};

// Where the user can learn more about a model — official maker pages where
// they exist, Hugging Face model cards where they don't have one canonical page.
const INFO_URLS: Record<string, string> = {
  'Gemma 3 1B': 'https://ai.google.dev/gemma',
  'Gemma 3 4B': 'https://ai.google.dev/gemma',
  'Llama 3.2 3B': 'https://www.llama.com/',
  'Llama 3.1 8B': 'https://www.llama.com/',
  'Phi-4-mini': 'https://huggingface.co/microsoft/Phi-4-mini-instruct',
  'Qwen 2.5 1.5B': 'https://qwenlm.github.io/',
  'Qwen 2.5 3B': 'https://qwenlm.github.io/',
  'Mistral Small': 'https://mistral.ai/news/mistral-small/',
};

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

  const ollamaTag = OLLAMA_TAGS[rec.modelName] ?? '';
  const infoUrl = INFO_URLS[rec.modelName];

  return (
    <main className="bg-slate-800 text-white min-h-screen px-6 sm:px-12 py-16">
      <div className="max-w-3xl mx-auto">
        <p className="text-emerald-400 text-sm mb-3 tracking-widest uppercase">Step 4 of 4</p>
        <h1 className="text-3xl sm:text-4xl md:text-5xl font-bold mb-12 leading-tight">
          We recommend{' '}
          {infoUrl ? (
            <a
              href={infoUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-emerald-400 hover:text-emerald-300 underline-offset-4 hover:underline transition-colors"
            >
              {rec.modelName}
            </a>
          ) : (
            <span className="text-emerald-400">{rec.modelName}</span>
          )}
        </h1>

        {/* Alternatives */}
        {rec.alternatives.length > 0 && (
          <div className="mb-12">
            <h3 className="text-lg font-semibold mb-3 text-slate-300">Alternatives</h3>
            <div className="space-y-2">
              {rec.alternatives.map((alt) => {
                const altInfoUrl = INFO_URLS[alt.name];
                return (
                  <div key={alt.name} className="bg-slate-900/50 border border-slate-700 rounded-lg p-4">
                    {altInfoUrl ? (
                      <a
                        href={altInfoUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="font-semibold text-emerald-400 hover:text-emerald-300 underline-offset-4 hover:underline transition-colors"
                      >
                        {alt.name}
                      </a>
                    ) : (
                      <p className="font-semibold text-emerald-400">{alt.name}</p>
                    )}
                    <p className="text-slate-400 text-sm">{alt.reason}</p>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Install paths — visual hierarchy: easiest first, most technical last */}
        <h2 className="text-2xl sm:text-3xl font-bold mb-6">How to install</h2>
        <div className="space-y-4">
          <VerdureWaitlist
            intentText={intent}
            modelName={rec.modelName}
            tier={tier}
          />
          <DesktopAppsPath />
          <OllamaPath modelName={rec.modelName} ollamaTag={ollamaTag} />
        </div>

        <div className="mt-12 text-center">
          <Link href="/start" className="text-slate-400 hover:text-white text-sm">
            ← Start over with different choices
          </Link>
        </div>
      </div>
    </main>
  );
}

/** Path 1 — Verdure waitlist. Most prominent visually. */
function VerdureWaitlist({
  intentText,
  modelName,
  tier,
}: {
  intentText: string;
  modelName: string;
  tier: Tier;
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

  return (
    <div className="bg-emerald-400 text-slate-900 rounded-xl p-6 shadow-lg">
      <div className="flex flex-wrap items-start justify-between gap-3 mb-3">
        <div>
          <p className="text-xs uppercase tracking-wider font-bold opacity-70">Easiest</p>
          <h3 className="text-2xl font-bold">Verdure — one-click install</h3>
        </div>
        <span className="bg-slate-900 text-emerald-400 text-xs px-3 py-1 rounded-full font-semibold">
          Coming soon
        </span>
      </div>
      <p className="text-slate-900/80 mb-4">
        Verdure handles everything — detection, download, configuration. We&rsquo;re launching soon. Join the waitlist and we&rsquo;ll email you the moment it&rsquo;s ready.
      </p>

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
            We save your name, email, and what you typed above so we can tell you when Verdure is ready and build the things people actually want.
          </p>
        </form>
      )}
    </div>
  );
}

/** Path 2 — LM Studio / Jan. Easier than terminal, no install help yet. */
function DesktopAppsPath() {
  return (
    <div className="bg-slate-900 border border-slate-700 rounded-xl p-6">
      <p className="text-xs uppercase tracking-wider font-bold text-slate-400 mb-1">
        Easier than the terminal
      </p>
      <h3 className="text-xl font-bold mb-2">Use LM Studio or Jan</h3>
      <p className="text-slate-300 mb-4">
        Open-source desktop apps. Open one, search for the model, click install.
      </p>
      <div className="flex flex-wrap gap-3">
        <a
          href="https://lmstudio.ai"
          target="_blank"
          rel="noopener noreferrer"
          className="border border-slate-600 text-white px-5 py-2.5 rounded-full text-sm font-semibold hover:border-emerald-400 transition"
        >
          Get LM Studio ↗
        </a>
        <a
          href="https://jan.ai"
          target="_blank"
          rel="noopener noreferrer"
          className="border border-slate-600 text-white px-5 py-2.5 rounded-full text-sm font-semibold hover:border-emerald-400 transition"
        >
          Get Jan ↗
        </a>
      </div>
    </div>
  );
}

/** Path 3 — Ollama. Most technical: copy-pasteable commands. */
function OllamaPath({ modelName, ollamaTag }: { modelName: string; ollamaTag: string }) {
  const [copied, setCopied] = useState(false);
  const cmd = ollamaTag ? `ollama run ${ollamaTag}` : `ollama run ${modelName.toLowerCase().replace(/\s+/g, '-')}`;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(cmd);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard API unavailable — fail silently; user can select & copy manually
    }
  };

  return (
    <div className="bg-slate-900 border border-slate-700 rounded-xl p-6">
      <p className="text-xs uppercase tracking-wider font-bold text-slate-400 mb-1">
        Technical
      </p>
      <h3 className="text-xl font-bold mb-2">Use Ollama</h3>
      <p className="text-slate-300 mb-4">
        Two commands in your terminal. Best if you&rsquo;re already comfortable with the command line.
      </p>
      <div className="space-y-2">
        <CommandLine label="Install Ollama" command="curl -fsSL https://ollama.com/install.sh | sh" />
        <CommandLine label={`Run ${modelName}`} command={cmd} onCopy={copy} copied={copied} />
      </div>
    </div>
  );
}

function CommandLine({
  label,
  command,
  onCopy,
  copied,
}: {
  label: string;
  command: string;
  onCopy?: () => void;
  copied?: boolean;
}) {
  return (
    <div>
      <p className="text-xs text-slate-500 mb-1">{label}</p>
      <div className="flex items-center gap-2 bg-slate-950 border border-slate-700 rounded-lg p-3 font-mono text-sm text-emerald-300">
        <code className="flex-1 overflow-x-auto whitespace-nowrap">{command}</code>
        {onCopy && (
          <button
            onClick={onCopy}
            className="text-xs text-slate-400 hover:text-emerald-400 px-2 py-1 rounded transition shrink-0"
          >
            {copied ? 'Copied!' : 'Copy'}
          </button>
        )}
      </div>
    </div>
  );
}
