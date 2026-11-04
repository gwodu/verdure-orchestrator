import type { Metadata } from 'next';
import Link from 'next/link';
import Pipeline from '@/components/Pipeline';
import CodeTabs from '@/components/CodeTabs';
import { API, LICENCES, SPEECH, TIERS } from '@/lib/sdk-content';
import { SDK_REQUEST_MAILTO } from '@/lib/links';

export const metadata: Metadata = {
  title: 'The Verdure SDK — how it works, the API, what we measured',
  description:
    'Speech to transcript to schema-valid JSON on the user’s own device. One library, one licence token, offline. The hello world in Kotlin, Rust and Python, the API, and every measured number.',
};

// The white paper, navigable: a sticky list of sections on the left, plain
// sections on the right, each short enough to read in a minute. The facts
// (API rows, samples, numbers) are data in lib/sdk-content.ts and tested there.
const SECTIONS = [
  ['what', 'What it is'],
  ['how', 'How it works'],
  ['hello', 'Hello world'],
  ['api', 'The API'],
  ['devices', 'Which model, which device'],
  ['measured', 'What we measured'],
  ['privacy', 'Privacy and licences'],
  ['not-yet', 'What is not done yet'],
  ['get', 'Get the SDK'],
] as const;

export default function Sdk() {
  return (
    <main className="max-w-6xl mx-auto px-5 sm:px-8 pt-12 sm:pt-16">
      <p className="text-mint-bright text-xs sm:text-sm tracking-[0.2em] uppercase mb-4">The Verdure SDK</p>
      <h1 className="text-3xl sm:text-5xl font-bold leading-tight max-w-3xl">
        Speech to transcript to structured JSON, on the device in front of your user.
      </h1>
      <p className="mt-5 text-lg sm:text-xl text-slate-300 max-w-3xl leading-relaxed">
        This page is the whole story: what the runtime does, the code you write, how it chooses a model, and the
        numbers we measured, including the ones that are not good enough yet.
      </p>

      <div className="mt-12 grid lg:grid-cols-[220px_1fr] gap-10">
        {/* Section nav: sticky on wide screens, a scrolling row on phones. */}
        <nav aria-label="Sections" className="lg:sticky lg:top-24 self-start min-w-0">
          <ol className="flex lg:flex-col gap-x-5 gap-y-2 max-w-full overflow-x-auto whitespace-nowrap pb-2 lg:pb-0 text-sm">
            {SECTIONS.map(([id, label], i) => (
              <li key={id}>
                <a href={`#${id}`} className="text-ink-soft hover:text-mint-bright">
                  <span className="text-mint-bright/70 mr-2">{i + 1}</span>{label}
                </a>
              </li>
            ))}
          </ol>
        </nav>

        <div className="space-y-20 min-w-0">
          <Section id="what" title="What it is">
            <p>
              Verdure is a runtime a company embeds in its own app. On the user&rsquo;s device, with no internet, it does
              three things:
            </p>
            <ol className="list-decimal pl-6 space-y-2">
              <li><b>Speech to text.</b> Microphone audio in, transcript events out.</li>
              <li><b>Text to a filled form.</b> You give it free text and a JSON Schema you wrote; it returns JSON that matches the schema, or an error that says exactly which fields were wrong.</li>
              <li><b>Search over documents already on the device.</b> You index documents; you ask a question; you get the best passages back.</li>
            </ol>
            <p>
              The company keeps its own app, screens and cloud. Verdure is the engine underneath. It never opens a
              network connection except to download model files from a server the company configured, and only
              when the company&rsquo;s policy allows that.
            </p>
            <p>
              It is written in Rust and ships as one library with whisper.cpp and llama.cpp compiled in, with
              bindings for Kotlin, Python and Rust today. The licence is a signed token verified offline. The models
              come from a signed manifest the company writes: Verdure never adds a model that is not listed.
            </p>
          </Section>

          <Section id="how" title="How it works">
            <div className="glass p-3 sm:p-6"><Pipeline /></div>
            <p>The flow your code drives, in order:</p>
            <ol className="list-decimal pl-6 space-y-2">
              <li><b>start</b> — verify the licence token, read the device: tier, free memory, free disk, cores. No network.</li>
              <li><b>loadManifest</b> — trust the signed list of bundles. A bundle is one set of model files for one task on certain device tiers.</li>
              <li><b>recommend</b> — the bundle this device can run now, for live speech. Everything passed over is explained in a sentence: not enough memory, not enough disk, too slow to keep pace.</li>
              <li><b>download</b> — every file, resumable, verified by SHA-256, shared between bundles that use the same model.</li>
              <li><b>load</b> — the engines into memory. Refused with the numbers if memory is short.</li>
              <li><b>transcribe</b> — push 16 kHz audio, get transcript events.</li>
              <li><b>extract</b> — the transcript plus your schema in, valid JSON out. The model only ever sees an outline of the schema and the rule &ldquo;never invent numbers, names or dates&rdquo;; the output is validated against the real schema, and one repair round is attempted before an error is returned.</li>
            </ol>
          </Section>

          <Section id="hello" title="Hello world">
            <p>
              The same flow in each language. The Kotlin one is what an Android app calls; it compiled and ran unchanged
              when we swapped a new model under a test integration, because the app never names a model.
            </p>
            <CodeTabs />
            <p className="text-ink-soft text-sm">
              Leave <code className="inline">keyterms</code> empty: a 40-word clinical list made recognition worse in our
              measurements. Use <code className="inline">replacements</code> for spellings you need fixed. Every call blocks;
              keep them off the main thread.
            </p>
          </Section>

          <Section id="api" title="The API">
            <p>Every method on <code className="inline">Verdure</code>. Kotlin spelling shown; Rust and Python use snake_case.</p>
            <div className="overflow-x-auto">
              <table className="doc">
                <thead><tr><th>Method</th><th>Does</th><th>Fails with</th></tr></thead>
                <tbody>
                  {API.map((r) => (
                    <tr key={r.method}>
                      <td><code className="inline">{r.method}</code></td>
                      <td>{r.does}</td>
                      <td>{r.fails ?? '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p className="text-ink-soft text-sm">
              Errors are typed and their messages are sentences for your log: <code className="inline">SchemaInvalid</code> lists each
              violation by JSON path; <code className="inline">NoBundle</code> says what did not fit and why. Show your user a sentence of your own.
            </p>
          </Section>

          <Section id="devices" title="Which model, which device">
            <p>
              Verdure sorts devices into six tiers by what they can honestly hold, and a bundle declares which tiers it is
              built for. The device has the last word: <code className="inline">recommend</code> reads free memory right now, not a spec sheet.
            </p>
            <div className="overflow-x-auto">
              <table className="doc">
                <thead><tr><th>Tier</th><th>Device</th><th>For example</th></tr></thead>
                <tbody>{TIERS.map((t) => <tr key={t.tier}><td>{t.tier}</td><td>{t.device}</td><td>{t.example}</td></tr>)}</tbody>
              </table>
            </div>
            <p>
              Memory is a rule, not a guess: a language model needs its file size times 1.5 plus 400 MB, the speech model
              its size plus 100 MB. Speed is a measurement the manifest carries per tier. For live speech, a bundle slower
              than 1.5× real time on this tier is passed over with the number as the reason; a tier we have not measured is
              not judged. For audio that is already recorded, <code className="inline">recommendFor(task, Pace.RECORDED)</code> allows
              the slower, better bundle.
            </p>
            <p>
              You can always pin a bundle yourself: <code className="inline">fit(id)</code> tells you whether it fits that device
              and at what speed, and <code className="inline">loadOvercommit</code> exists for a device you have measured yourself.
            </p>
          </Section>

          <Section id="measured" title="What we measured">
            <p>
              Ten real two-speaker GP consultations (PriMock57, 12,000 reference words), scored on word error rate and on
              a list of a hundred clinical terms, run through the SDK&rsquo;s own <code className="inline">transcribe()</code> on an
              8 GB Intel Mac and on a Pixel 8a inside a test Android app. Speed is processing time per second of speech, fed
              flat out; the phone figure is the sustained, throttled one.
            </p>
            <div className="overflow-x-auto">
              <table className="doc">
                <thead><tr><th>Speech model</th><th>Bundle</th><th>Download</th><th>Word error</th><th>Clinical terms missed</th><th>Speed, Intel Mac</th><th>Speed, Pixel 8a</th></tr></thead>
                <tbody>
                  {SPEECH.map((r) => (
                    <tr key={r.bundle}><td>{r.model}</td><td><code className="inline">{r.bundle}</code></td><td>{r.download}</td><td>{r.wer}</td><td>{r.missed}</td><td>{r.mac}</td><td>{r.pixel}</td></tr>
                  ))}
                </tbody>
              </table>
            </div>
            <ul className="list-disc pl-6 space-y-2">
              <li><b>Quality travels.</b> The phone produced the same transcript quality as the laptop, file for file, with the same words missed.</li>
              <li><b>Memory on the Pixel 8a:</b> the base bundle 683 MB idle and 952 MB peak; the small bundle 635 MB idle and 1,143 MB peak. No out-of-memory kill in 79 minutes of audio.</li>
              <li><b>Download and load:</b> 979 MB in 54 s over Wi-Fi, verified; load 15 s (small) and 1.2 s (base). Two bundles that share a model download it once.</li>
              <li><b>Keyterms hurt.</b> Feeding a 40-word vocabulary list to the recogniser doubled the error rate. It is documented as experimental and off by default.</li>
              <li><b>A medical fine-tune was worse.</b> A clinical Whisper medium fine-tuned on African-accented read speech scored 83 % word error on these consultations as converted. We do not ship it.</li>
            </ul>
            <p className="text-ink-soft text-sm">
              Full tables and method in the repository&rsquo;s <code className="inline">TEST_REPORT.md</code> and <code className="inline">knowledge/26-sdk.md</code>; they come with the SDK.
            </p>
          </Section>

          <Section id="privacy" title="Privacy and licences">
            <p>
              What leaves the device: nothing, except the model files coming in, from a host you name in the policy, over
              https only. No telemetry, no accounts, no counters unless you turn local counters on for yourself. Storage is
              encrypted by default and <code className="inline">delete(All)</code> destroys the key before the files. Verdure holds no
              user data on any server, so it is not a processor of it.
            </p>
            <p>
              The licence token is verified offline against a public key; a token names the vendor, the seats, the bundles
              and the model licences the vendor has acknowledged. Those model licences are listed, not discovered:
            </p>
            <div className="overflow-x-auto">
              <table className="doc">
                <thead><tr><th>Component</th><th>Licence</th><th>From</th></tr></thead>
                <tbody>{LICENCES.map((l) => <tr key={l.what}><td>{l.what}</td><td>{l.licence}</td><td>{l.from}</td></tr>)}</tbody>
              </table>
            </div>
            <p className="text-ink-soft text-sm">
              All permissive and shippable in a commercial product. A bundle whose model needs an acknowledgement (Llama, Gemma
              terms) is refused until the token carries it; attribution strings a licence requires are handed to your app by
              <code className="inline"> capabilities()</code>.
            </p>
          </Section>

          <Section id="not-yet" title="What is not done yet">
            <ul className="list-disc pl-6 space-y-2">
              <li><b>Extraction on the smallest language model is not good enough.</b> The 0.5B model returned schema-valid facts on 3 of 10 one-minute windows on the phone; the rest failed validation. The 1.5B bundle is the next measurement. Until then, extraction on a phone is a demonstration, not a product.</li>
              <li><b>Speed is measured on two devices.</b> A Pixel 8a and an 8 GB Intel Mac. Other tiers are not judged for speed until we measure them.</li>
              <li><b>Windows are fixed at 12 seconds.</b> Sentences get cut; voice-activity windowing is next.</li>
              <li><b>Grammar-constrained decoding is built but off</b>, after the engine aborted on a multi-byte token in an earlier app. Validation plus one repair round stands in for it.</li>
              <li><b>Bindings:</b> Kotlin, Python and Rust today. Swift and a C header are generated by the same tool and not yet packaged.</li>
            </ul>
          </Section>

          <Section id="get" title="Get the SDK">
            <p>
              Write to us and say which platform you build for. You receive the Android zip (library, Kotlin binding, a
              ten-year sandbox licence, a signed example manifest with real models, the guide and a changelog) or the
              repository for Rust and Python, and a person who answers questions the same day.
            </p>
            <div className="flex flex-wrap gap-4">
              <a href={SDK_REQUEST_MAILTO} className="btn-primary">Email for the test kit →</a>
              <Link href="/try" className="btn-secondary">Try the app first</Link>
            </div>
          </Section>
        </div>
      </div>
    </main>
  );
}

function Section({ id, title, children }: { id: string; title: string; children: React.ReactNode }) {
  return (
    <section id={id} className="anchor">
      <h2 className="text-2xl sm:text-3xl font-bold mb-5">{title}</h2>
      <div className="space-y-5 text-slate-300 leading-relaxed text-base sm:text-lg">{children}</div>
    </section>
  );
}
