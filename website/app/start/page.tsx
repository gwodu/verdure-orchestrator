'use client';

import { useState, type FormEvent } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { saveIntent } from '@/lib/session';
import { INTENT_CARDS, type IntentCard } from '@verdure/decider';

export default function IntentScreen() {
  const [text, setText] = useState('');
  const router = useRouter();

  // Picking a card feeds its canonical example phrase into the flow; the
  // classifier maps it to the card's (modality, task). Free text routes the
  // same way, for the long tail (video, music, editing, RAG, …).
  const go = (intent: string) => {
    const trimmed = intent.trim();
    if (!trimmed) return;
    saveIntent(trimmed);
    router.push('/start/device');
  };

  const pick = (card: IntentCard) => go(card.example);
  const submit = (e: FormEvent) => {
    e.preventDefault();
    go(text);
  };

  return (
    <main className="bg-slate-800 text-white min-h-screen px-6 sm:px-12 py-16">
      <div className="max-w-3xl mx-auto">
        <p className="text-emerald-400 text-sm mb-3 tracking-widest uppercase">Step 1 of 4</p>
        <h1 className="text-4xl sm:text-5xl md:text-6xl font-bold mb-3 leading-tight">
          What do you want to do?
        </h1>
        <p className="text-slate-300 text-lg mb-10">
          Pick the closest match. We&rsquo;ll find AI that runs on the computer you already have — private and free — and show you exactly how to start.
        </p>

        <div className="grid sm:grid-cols-2 gap-4">
          {INTENT_CARDS.map((card) => (
            <button
              key={card.id}
              onClick={() => pick(card)}
              className="text-left p-6 bg-slate-900 border border-slate-700 rounded-xl hover:border-emerald-400 hover:bg-slate-900/60 transition group"
            >
              <div className="text-3xl mb-3" aria-hidden="true">{card.emoji}</div>
              <h2 className="text-xl font-semibold mb-1 text-white group-hover:text-emerald-400 transition-colors">
                {card.title}
              </h2>
              <p className="text-slate-400 text-sm">{card.description}</p>
            </button>
          ))}
        </div>

        {/* Free-text fallback — the long tail, and the path the future LLM classifier improves. */}
        <form onSubmit={submit} className="mt-10 border-t border-slate-700 pt-8">
          <label htmlFor="intent" className="block text-slate-300 text-lg font-semibold mb-3">
            Something else? Describe it in your own words.
          </label>
          <textarea
            id="intent"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="e.g. make a picture for my blog, write a song, summarize a PDF…"
            className="w-full min-h-[96px] p-4 bg-slate-900 border border-slate-700 rounded-xl text-white placeholder-slate-500 focus:border-emerald-400 focus:outline-none text-lg resize-y"
          />
          <div className="flex items-center gap-4 mt-4">
            <button
              type="submit"
              disabled={!text.trim()}
              className="bg-emerald-400 text-slate-900 px-8 py-4 rounded-full text-lg font-semibold hover:bg-emerald-300 transition disabled:opacity-40 disabled:cursor-not-allowed"
            >
              Continue →
            </button>
            <Link href="/" className="text-slate-400 hover:text-white text-sm">
              ← Back home
            </Link>
          </div>
        </form>
      </div>
    </main>
  );
}
