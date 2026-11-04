import Link from 'next/link';
import Pipeline from '@/components/Pipeline';
import KeepPosted from '@/components/KeepPosted';
import { LINKEDIN, MACOS_DMG } from '@/lib/links';

// The home page has one job: a company deciding whether to put Verdure in its
// app should know in ten seconds what it does, and be one click from running
// it. Everything else on the page earns the next sentence honestly
// (skills/copywriting.md): what it does, where it runs, what we measured,
// then the app for anyone who wants to feel it on their own device.
export default function Home() {
  return (
    <main className="max-w-6xl mx-auto px-5 sm:px-8">
      {/* Hero */}
      <section className="pt-20 sm:pt-28 pb-16 sm:pb-20">
        <p className="rise text-mint-bright text-xs sm:text-sm tracking-[0.2em] uppercase mb-5">The Verdure SDK</p>
        <h1 className="rise text-4xl sm:text-6xl md:text-7xl font-bold leading-[1.05] tracking-tight max-w-4xl">
          Run your local models with a few lines of code.
        </h1>
        <p className="rise-2 mt-7 text-lg sm:text-2xl text-slate-300 leading-relaxed max-w-3xl">
          Verdure is a runtime your app embeds. On the user&rsquo;s own phone or laptop it turns speech into a
          transcript, and the transcript into JSON that matches your schema. Offline. Nothing leaves the device.
        </p>
        <div className="rise-3 mt-10 flex flex-wrap items-center gap-4">
          <a href="/sdk#hello" className="btn-primary text-base sm:text-lg">See the hello world →</a>
          <Link href="/sdk" className="btn-secondary text-base sm:text-lg">Read how it works</Link>
        </div>
        <p className="rise-3 mt-5 text-ink-soft">
          Not a developer? <Link href="/try" className="text-mint-bright hover:underline">Try Verdure on your own Mac, iPhone or Android.</Link>
        </p>
      </section>

      {/* What the app gets */}
      <section className="grid md:grid-cols-3 gap-5 mb-20">
        {[
          {
            t: 'Speech to text',
            d: 'Microphone audio in, transcript events out as the person speaks. The speech model is chosen for the device, not for the demo.',
          },
          {
            t: 'Text to a filled form',
            d: 'Hand Verdure your JSON Schema. You get JSON that validates against it, or an error that names each field that was wrong.',
          },
          {
            t: 'The right model for the device',
            d: 'Verdure reads the memory, the disk and the measured speed of the phone it is on, then picks the bundle that runs live, or says in a sentence why not.',
          },
        ].map((c) => (
          <div key={c.t} className="glass p-6 sm:p-7">
            <h2 className="text-xl font-bold text-mint-bright mb-2">{c.t}</h2>
            <p className="text-slate-300 leading-relaxed">{c.d}</p>
          </div>
        ))}
      </section>

      {/* The picture */}
      <section className="mb-20">
        <div className="glass p-4 sm:p-8">
          <Pipeline />
        </div>
        <p className="mt-4 text-ink-soft text-sm sm:text-base max-w-3xl">
          One library (whisper.cpp and llama.cpp compiled in), one licence token verified offline, one signed
          manifest that says which model files exist and where they come from. Your app keeps its screens and its
          cloud. Verdure is the engine underneath.
        </p>
      </section>

      {/* Measured, not promised */}
      <section className="mb-20">
        <h2 className="text-2xl sm:text-3xl font-bold mb-2">Measured, not promised</h2>
        <p className="text-ink-soft mb-8 max-w-3xl">
          Every number here comes from a run we can repeat: ten real two-speaker GP consultations, scored by word
          error and by the clinical terms a note is made of, on an 8 GB Intel Mac and a Pixel 8a.
        </p>
        <div className="grid sm:grid-cols-3 gap-5">
          <Stat big="1 in 6" small="clinical terms missed with Whisper small, down from 1 in 3.6 with Whisper base. Same result on the phone as on the laptop." />
          <Stat big="54 s" small="to download the 979 MB clinical bundle on a Pixel 8a over Wi-Fi, verified by hash. 15 s to load it; 1.2 s for the smaller one." />
          <Stat big="1.25×" small="real time for the live-speech bundle on a Pixel 8a, flat out and throttled. The larger model runs at 3.2×, so Verdure picks it only for recordings." />
        </div>
        <p className="mt-6 text-ink-soft">
          <Link href="/sdk#measured" className="text-mint-bright hover:underline">All the numbers, including what is not done yet →</Link>
        </p>
      </section>

      {/* Who it is for */}
      <section className="mb-20 grid md:grid-cols-[1.2fr_1fr] gap-8 items-start">
        <div>
          <h2 className="text-2xl sm:text-3xl font-bold mb-4">For apps where the data has to stay in the room</h2>
          <p className="text-slate-300 leading-relaxed mb-4">
            A clinician dictating between patients. A lesson being recorded for a teacher&rsquo;s coaching. A community
            health worker filling a form on a 4 GB phone after a home visit. Their words should not have to leave the
            building to become a note.
          </p>
          <p className="text-slate-300 leading-relaxed">
            Verdure runs on the phones and laptops those people already have, down to a 4 GB phone for form
            filling and an 8 GB Intel laptop for a full scribe. Your cloud stays your fallback, through a hook you
            implement; Verdure never opens that connection itself.
          </p>
        </div>
        <ul className="glass p-6 space-y-3 text-slate-300">
          <li><span className="text-mint-bright font-semibold">Clinical scribes</span> — offline transcript and facts, uploaded when the connection returns.</li>
          <li><span className="text-mint-bright font-semibold">Education</span> — a tutor over lesson packs already on the device; coaching from a recorded lesson, never the audio.</li>
          <li><span className="text-mint-bright font-semibold">Community health</span> — speak to fill the form, on the cheapest phones in the field.</li>
        </ul>
      </section>

      {/* Try the app */}
      <section className="mb-20">
        <div className="glass p-6 sm:p-10 grid md:grid-cols-[1fr_auto] gap-6 items-center">
          <div>
            <h2 className="text-2xl sm:text-3xl font-bold mb-2">Want to feel it first?</h2>
            <p className="text-slate-300 leading-relaxed max-w-2xl">
              Verdure the app runs on the same engine: a private chat with a model on your own device, dictation
              into it, and nothing sent anywhere. Download it for your Mac now, or ask for a test build on iPhone
              or Android.
            </p>
          </div>
          <div className="flex flex-col gap-3">
            <a href={MACOS_DMG} className="btn-primary justify-center">Download for macOS</a>
            <Link href="/try" className="btn-secondary justify-center">iPhone, iPad or Android</Link>
          </div>
        </div>
      </section>

      {/* Keep me posted */}
      <section className="mb-20 max-w-3xl">
        <KeepPosted
          intent="homepage: keep me posted"
          title="Keep me posted"
          blurb="A short note when there is something new to try: new builds, new platforms. No newsletter, no tracking."
          button="Keep me posted"
        />
      </section>

      {/* Founder */}
      <section className="mb-10 flex flex-col sm:flex-row gap-6 sm:gap-10 items-center sm:items-start">
        <img
          src="/Founders-Portrait.jpg"
          alt="Gesi Wodu"
          className="w-36 h-44 object-cover object-top rounded-2xl border border-[rgba(76,216,173,0.3)]"
        />
        <div>
          <h2 className="text-xl sm:text-2xl font-bold mb-1">Built by Gesi Wodu</h2>
          <p className="text-slate-300 leading-relaxed max-w-2xl">
            Computer science at the University of Toronto, then this: AI that is affordable, power-efficient and
            runs where the person is, for people doing good work. If you are building for them, I would like to
            hear from you.
          </p>
          <a href={LINKEDIN} target="_blank" rel="noopener noreferrer" className="inline-block mt-3 text-mint-bright hover:underline">
            Connect on LinkedIn →
          </a>
        </div>
      </section>
    </main>
  );
}

function Stat({ big, small }: { big: string; small: string }) {
  return (
    <div className="glass p-6">
      <p className="text-4xl sm:text-5xl font-bold text-mint-bright mb-2">{big}</p>
      <p className="text-slate-300 text-sm sm:text-base leading-relaxed">{small}</p>
    </div>
  );
}
