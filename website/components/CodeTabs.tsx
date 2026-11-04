'use client';

import { useState } from 'react';
import { SAMPLES, type Sample } from '@/lib/sdk-content';

// Three languages, one flow. The code is data (lib/sdk-content.ts) so a test
// can hold it to the API table; this component only switches between them.
export default function CodeTabs() {
  const [active, setActive] = useState<Sample['id']>('kotlin');
  const [copied, setCopied] = useState(false);
  const sample = SAMPLES.find((s) => s.id === active)!;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(sample.code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard unavailable: the text is selectable anyway */
    }
  };

  return (
    <div className="glass p-3 sm:p-4">
      <div className="flex flex-wrap items-center justify-between gap-2 mb-3">
        <div role="tablist" aria-label="Language" className="flex gap-1">
          {SAMPLES.map((s) => (
            <button
              key={s.id}
              role="tab"
              aria-selected={s.id === active}
              onClick={() => { setActive(s.id); setCopied(false); }}
              className={`px-3 py-1.5 rounded-full text-sm font-semibold transition-colors ${
                s.id === active ? 'bg-mint text-on-mint' : 'text-ink-soft hover:text-ink'
              }`}
            >
              {s.label}
            </button>
          ))}
        </div>
        <button onClick={copy} className="text-sm text-ink-soft hover:text-mint-bright">
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      <pre className="code" tabIndex={0}><code>{sample.code}</code></pre>
    </div>
  );
}
