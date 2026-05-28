'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { getIntent, getTier } from '@/lib/session';
import { findBucket, type Tier } from '@/lib/tiers';
import { classifyIntent, type Intent } from '@/lib/intent';

const INTENT_LABEL: Record<Intent, string> = {
  creative: 'Creative writing',
  code: 'Code & technical',
  multilingual: 'Multilingual',
  general: 'General chat / Q&A',
};

export default function ConfirmScreen() {
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

  if (!intent || !tier) return null;

  const bucket = findBucket(tier);
  const classified = classifyIntent(intent);

  return (
    <main className="bg-slate-800 text-white min-h-screen flex flex-col items-center justify-center px-6 sm:px-12 py-16">
      <div className="max-w-2xl w-full">
        <p className="text-emerald-400 text-sm mb-3 tracking-widest uppercase">Step 3 of 4</p>
        <h1 className="text-3xl sm:text-4xl md:text-5xl font-bold mb-8">
          Here&rsquo;s what we understood.
        </h1>

        <div className="space-y-4 mb-10">
          <div className="bg-slate-900 border border-slate-700 rounded-xl p-6">
            <p className="text-slate-400 text-xs uppercase tracking-wider mb-2">What you want</p>
            <p className="text-lg text-white mb-3 italic">&ldquo;{intent}&rdquo;</p>
            <p className="text-emerald-400 text-sm">
              Category: <span className="font-semibold">{INTENT_LABEL[classified]}</span>
            </p>
          </div>
          <div className="bg-slate-900 border border-slate-700 rounded-xl p-6">
            <p className="text-slate-400 text-xs uppercase tracking-wider mb-2">Your device</p>
            <p className="text-lg text-white">{bucket?.label}</p>
            <p className="text-emerald-400 text-sm font-medium">{bucket?.ramRange}</p>
          </div>
        </div>

        <div className="flex flex-col sm:flex-row gap-4">
          <button
            onClick={() => router.push('/start/recommendation')}
            className="bg-emerald-400 text-slate-900 px-8 py-4 rounded-full text-lg font-semibold hover:bg-emerald-300 transition"
          >
            Looks right →
          </button>
          <button
            onClick={() => router.push('/start')}
            className="border border-slate-600 text-white px-8 py-4 rounded-full text-lg font-semibold hover:border-emerald-400 transition"
          >
            Start over
          </button>
        </div>
      </div>
    </main>
  );
}
