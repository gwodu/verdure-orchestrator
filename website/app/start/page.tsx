'use client';

import { useState, type FormEvent } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { saveIntent } from '@/lib/session';

export default function IntentScreen() {
  const [text, setText] = useState('');
  const router = useRouter();

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const trimmed = text.trim();
    if (!trimmed) return;
    saveIntent(trimmed);
    router.push('/start/device');
  };

  return (
    <main className="bg-slate-800 text-white min-h-screen flex flex-col items-center justify-center px-6 sm:px-12 py-16">
      <div className="max-w-2xl w-full">
        <p className="text-emerald-400 text-sm mb-3 tracking-widest uppercase">Step 1 of 4</p>
        <h1 className="text-4xl sm:text-5xl md:text-6xl font-bold mb-6 leading-tight">
          What do you want your AI to help with?
        </h1>
        <p className="text-slate-300 text-lg mb-10">
          Describe it in your own words. Examples: &ldquo;help me write fantasy novels,&rdquo; &ldquo;debug Rust code at work,&rdquo; &ldquo;Spanish homework help for my kid.&rdquo;
        </p>
        <form onSubmit={submit} className="space-y-6">
          <textarea
            autoFocus
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="I want to..."
            className="w-full min-h-[140px] p-4 bg-slate-900 border border-slate-700 rounded-xl text-white placeholder-slate-500 focus:border-emerald-400 focus:outline-none text-lg resize-y"
          />
          <div className="flex items-center gap-4">
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
