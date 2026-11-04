'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { DEVICE_CARDS, type DeviceCard } from '@verdure/decider';
import { getIntent, saveDevice } from '@/lib/session';

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

  const pick = (card: DeviceCard) => {
    saveDevice(card.id);
    router.push('/start/confirm');
  };

  if (!ready) return null;

  return (
    <main className="bg-paper text-ink min-h-screen px-6 sm:px-12 py-16">
      <div className="max-w-3xl mx-auto">
        <p className="text-green text-sm mb-3 tracking-widest uppercase">Step 2 of 4</p>
        <h1 className="text-4xl sm:text-5xl md:text-6xl font-bold mb-3 leading-tight">
          What kind of computer will you use?
        </h1>
        <p className="text-ink-2 text-lg mb-10">
          Pick whichever one sounds the most like yours.
        </p>
        <div className="grid sm:grid-cols-2 gap-4">
          {DEVICE_CARDS.map((card) => (
            <button
              key={card.id}
              onClick={() => pick(card)}
              className="text-left p-6 bg-white border border-line rounded-xl hover:border-green hover:bg-white/60 transition group"
            >
              <div className="text-3xl mb-3" aria-hidden="true">{card.emoji}</div>
              <h2 className="text-xl font-semibold mb-1 text-ink group-hover:text-green-dark transition-colors">
                {card.title}
              </h2>
              <p className="text-ink-soft text-sm">{card.description}</p>
            </button>
          ))}
        </div>
        <div className="mt-10">
          <button
            onClick={() => router.push('/start')}
            className="text-ink-soft hover:text-ink text-sm"
          >
            ← Back
          </button>
        </div>
      </div>
    </main>
  );
}
