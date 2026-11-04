import Link from 'next/link';
import { CONTACT_EMAIL, LINKEDIN, MACOS_DMG } from '@/lib/links';

// The home page has one job: anyone, technical or not, knows in one glance
// what Verdure is — AI that runs on your own device, offline — and sees the
// one thing to do next. The uses are a list you open, not a wall you scroll;
// the developers get one short section and a door. Plain words throughout:
// the reader may be a clinic owner or a truck driver, not an engineer
// (skills/design.md, "Assume zero technical knowledge").
export default function Home() {
  return (
    <main className="max-w-5xl mx-auto px-5 sm:px-8">
      {/* What we are */}
      <section className="pt-20 sm:pt-32 pb-20 sm:pb-28 max-w-3xl">
        <h1 className="text-[2.6rem] leading-[1.05] sm:text-6xl md:text-7xl font-semibold tracking-tight">
          Run your own AI. Offline, on your own device.
        </h1>
        <p className="mt-7 text-lg sm:text-2xl text-ink-2 leading-relaxed">
          Verdure runs AI on the phone or computer you already have. What you say and type stays on it.
          No account. No internet needed.
        </p>
        <div className="mt-9 flex flex-wrap items-center gap-x-6 gap-y-3">
          <Link href="/try" className="btn-primary text-base sm:text-lg">Try the app</Link>
          <Link href="/developers" className="btn-text text-base sm:text-lg">For developers →</Link>
        </div>
      </section>

      {/* What it does */}
      <section className="pb-20 sm:pb-28 grid sm:grid-cols-3 gap-x-8 gap-y-10">
        <Point title="Ask it anything.">
          Like any AI chat, except the answer is made on your device, not on someone else&rsquo;s server.
        </Point>
        <Point title="Speak instead of typing.">
          Say it, and it becomes text. The recording never leaves your device.
        </Point>
        <Point title="Nothing to set up.">
          Open it. Verdure picks the AI that fits your device, tells you how big it is, and gets it ready.
        </Point>
      </section>

      {/* Where it fits */}
      <section className="pb-20 sm:pb-28">
        <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-3">Where it fits</h2>
        <p className="text-ink-2 text-lg mb-8 max-w-2xl">
          One engine, many jobs. These are the ones we are working on. Open one to read more.
        </p>
        <div className="space-y-2.5 max-w-3xl">
          <Row title="Doctors and clinics" lead="A note written while you talk with the patient, without the conversation leaving the room.">
            <p>
              This is the use we have measured most. On an ordinary phone and an 8 GB laptop, Verdure turns a
              consultation into text and misses about one clinical term in six, with the same quality on both.
              The transcript is then shaped into the fields of your note, on the device, and handed to your
              records system when you are ready.
            </p>
            <p>
              Turning the transcript into a finished note is further behind than the transcript itself. We say
              exactly how far on the <Link href="/developers#measured" className="link">developers page</Link>.
            </p>
          </Row>
          <Row title="Field teams and forms" lead="Say what you saw, and the form is drafted for you, on a cheap phone, with no signal.">
            <p>
              Built for the health worker on a home visit and anyone who fills the same form twenty times a day.
              Verdure runs on phones with 4 GB of memory. Everything waits on the phone until there is a
              connection, if there ever needs to be one.
            </p>
          </Row>
          <Row title="Schools and teachers" lead="A tutor that works from the lesson material on the device, and coaching from a recorded lesson that stays in the school.">
            <p>
              The same engine answers questions from documents already on the device, and turns a recorded lesson
              into text a coach can work from, without the recording going anywhere.
            </p>
          </Row>
          <Row title="Trades, drivers, small businesses" lead="Notes, messages and questions handled by voice, in the cab or on the shop floor, with no signal.">
            <p>
              This is what the app does today: a private chat with an AI on your own device, and dictation into it.
              Open it on your Mac, or ask for a phone test build on the <Link href="/try" className="link">try page</Link>.
            </p>
          </Row>
        </div>
      </section>

      {/* The app */}
      <section className="pb-20 sm:pb-28">
        <div className="card p-7 sm:p-10 grid md:grid-cols-[1fr_auto] gap-7 items-center">
          <div>
            <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-3">There is also the app.</h2>
            <p className="text-ink-2 text-lg leading-relaxed max-w-2xl">
              Verdure is an app you can use today. A private chat with an AI on your own device, and dictation
              into it. Free, no account.
            </p>
          </div>
          <div className="flex flex-col gap-3 sm:min-w-[220px]">
            <a href={MACOS_DMG} className="btn-primary">Download for Mac</a>
            <Link href="/try" className="btn-text justify-center">iPhone or Android →</Link>
          </div>
        </div>
      </section>

      {/* Developers */}
      <section className="pb-20 sm:pb-28 max-w-3xl">
        <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-3">For developers</h2>
        <p className="text-ink-2 text-lg leading-relaxed mb-5">
          Put Verdure inside your own app. A few lines of code give it speech to text and text to a filled form,
          running offline on your user&rsquo;s device. The developers page has the code, how it chooses a model
          for each device, every number we measured, and how to get the test kit.
        </p>
        <Link href="/developers" className="btn-text text-lg !pl-0">See how it works →</Link>
      </section>

      {/* Talk to us */}
      <section className="pb-10 max-w-3xl">
        <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-3">Talk to us</h2>
        <p className="text-ink-2 text-lg leading-relaxed">
          Want Verdure for your clinic, your team or your product? Write to{' '}
          <a href={`mailto:${CONTACT_EMAIL}`} className="link">{CONTACT_EMAIL}</a>. You will hear back from the
          person who built it.
        </p>
        <div className="mt-8 flex items-center gap-5">
          <img
            src="/Founders-Portrait.jpg"
            alt="Gesi Wodu"
            className="w-16 h-16 rounded-full object-cover object-top"
          />
          <div className="text-ink-2">
            <p className="font-semibold text-ink">Gesi Wodu, founder</p>
            <a href={LINKEDIN} target="_blank" rel="noopener noreferrer" className="link text-sm">LinkedIn</a>
          </div>
        </div>
      </section>
    </main>
  );
}

function Point({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h2 className="text-xl font-semibold tracking-tight mb-2">{title}</h2>
      <p className="text-ink-2 leading-relaxed">{children}</p>
    </div>
  );
}

function Row({ title, lead, children }: { title: string; lead: string; children: React.ReactNode }) {
  return (
    <details className="row">
      <summary>
        <span>
          <span className="font-semibold text-ink">{title}</span>
          <span className="block text-ink-2 mt-0.5">{lead}</span>
        </span>
      </summary>
      <div className="text-ink-2 leading-relaxed space-y-3">{children}</div>
    </details>
  );
}
