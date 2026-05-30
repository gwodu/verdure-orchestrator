'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { DEVICE_BUCKETS, type Tier } from '@/lib/tiers';
import { getIntent, saveTier } from '@/lib/session';

export default function DeviceScreen() {
  const router = useRouter();
  const [ready, setReady] = useState(false);

  // Redirect to step 1 if the user lands here without typing an intent.
  useEffect(() => {
    if (!getIntent()) {
      router.replace('/start');
      return;
    }
    setReady(true);
  }, [router]);

  const pick = (tier: Tier) => {
    saveTier(tier);
    router.push('/start/confirm');
  };

  if (!ready) return null;

  return (
    <main className="bg-slate-800 text-white min-h-screen px-6 sm:px-12 py-16">
      <div className="max-w-4xl mx-auto">
        <p className="text-emerald-400 text-sm mb-3 tracking-widest uppercase">Step 2 of 4</p>
        <h1 className="text-4xl sm:text-5xl md:text-6xl font-bold mb-3 leading-tight">
          What kind of device will run it?
        </h1>
        <p className="text-slate-300 text-lg mb-10">
          Pick the closest match. We&rsquo;ll recommend a model that runs comfortably on it.
        </p>
        <div className="grid sm:grid-cols-2 gap-4">
          {DEVICE_BUCKETS.map((b) => (
            <button
              key={b.id}
              onClick={() => pick(b.id)}
              className="text-left p-6 bg-slate-900 border border-slate-700 rounded-xl hover:border-emerald-400 hover:bg-slate-900/60 transition group"
            >
              <h3 className="text-xl font-semibold mb-1 text-white group-hover:text-emerald-400 transition-colors">
                {b.label}
              </h3>
              <p className="text-slate-400 text-sm mb-2">{b.description}</p>
              <p className="text-emerald-400 text-sm font-medium mb-2">{b.ramRange}</p>
              <p className="text-slate-500 text-xs">{b.example}</p>
            </button>
          ))}
        </div>
        <div className="mt-10">
          <button
            onClick={() => router.push('/start')}
            className="text-slate-400 hover:text-white text-sm"
          >
            ← Back
          </button>
        </div>
      </div>
    </main>
  );
}
