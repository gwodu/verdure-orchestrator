'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { getIntent, getDevice } from '@/lib/session';
import { findCard, type DeviceCard, classifyIntent, taskLabel } from '@verdure/decider';

export default function ConfirmScreen() {
  const router = useRouter();
  const [intent, setIntent] = useState<string | null>(null);
  const [card, setCard] = useState<DeviceCard | null>(null);

  useEffect(() => {
    const i = getIntent();
    const id = getDevice();
    const c = id ? findCard(id) : undefined;
    if (!i || !c) {
      router.replace('/start');
      return;
    }
    setIntent(i);
    setCard(c);
  }, [router]);

  if (!intent || !card) return null;

  const label = taskLabel(classifyIntent(intent));
  const labelDisplay = label.charAt(0).toUpperCase() + label.slice(1);

  return (
    <main className="bg-paper text-ink min-h-screen flex flex-col items-center justify-center px-6 sm:px-12 py-16">
      <div className="max-w-2xl w-full">
        <p className="text-green text-sm mb-3 tracking-widest uppercase">Step 3 of 4</p>
        <h1 className="text-3xl sm:text-4xl md:text-5xl font-bold mb-8">
          Here&rsquo;s what we understood.
        </h1>

        <div className="space-y-4 mb-10">
          <div className="bg-white border border-line rounded-xl p-6">
            <p className="text-ink-soft text-xs uppercase tracking-wider mb-2">What you want</p>
            <p className="text-lg text-ink mb-3 italic">&ldquo;{intent}&rdquo;</p>
            <p className="text-green text-sm">
              We&rsquo;ll help you with: <span className="font-semibold">{labelDisplay}</span>
            </p>
          </div>
          <div className="bg-white border border-line rounded-xl p-6">
            <p className="text-ink-soft text-xs uppercase tracking-wider mb-2">Your computer</p>
            <p className="text-lg text-ink">
              <span aria-hidden="true">{card.emoji}</span> {card.title}
            </p>
            <p className="text-ink-soft text-sm">{card.description}</p>
          </div>
        </div>

        <div className="flex flex-col sm:flex-row gap-4">
          <button
            onClick={() => router.push('/start/recommendation')}
            className="bg-green text-white px-8 py-4 rounded-full text-lg font-semibold hover:bg-green-dark transition"
          >
            Looks right →
          </button>
          <button
            onClick={() => router.push('/start')}
            className="border border-line text-ink px-8 py-4 rounded-full text-lg font-semibold hover:border-green transition"
          >
            Start over
          </button>
        </div>
      </div>
    </main>
  );
}
