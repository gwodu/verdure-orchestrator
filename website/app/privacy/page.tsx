import type { Metadata } from 'next';
import Link from 'next/link';

// Verdure's privacy policy. Required as a public URL by the App Store and Google
// Play, but written first as an honest description of what the software does —
// the claim "your data stays on your device" is the product (knowledge/06), so a
// policy that overstated it would damage the thing it is meant to support.
//
// Every network call the app can make is listed. If a future change adds one,
// this page changes in the same commit, or the policy has become a lie.
export const metadata: Metadata = {
  title: 'Privacy — Verdure',
  description:
    'Verdure runs AI on your own device. Your conversations never leave it. No telemetry, no accounts, no tracking.',
};

// Contact is a personal address on purpose: verdureai.com has no MX records, so
// privacy@verdureai.com would BOUNCE, and a deletion request that silently fails
// is worse than an unpolished address. Swap this the moment domain mail exists —
// app stores and users both rely on it working.
//
// Last substantive review of this page's claims against the shipping code.
const UPDATED = '14 September 2026';

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mb-12">
      <h2 className="text-2xl sm:text-3xl font-bold text-emerald-400 mb-4">{title}</h2>
      <div className="text-slate-200 text-base sm:text-lg leading-relaxed space-y-4">
        {children}
      </div>
    </section>
  );
}

export default function Privacy() {
  return (
    <div className="bg-slate-800 text-white min-h-screen">
      <main className="max-w-3xl mx-auto px-6 sm:px-12 py-16 sm:py-24">
        <Link href="/" className="text-emerald-400 hover:text-emerald-300 text-sm">
          ← verdureai.com
        </Link>

        <h1 className="text-4xl sm:text-5xl font-bold text-emerald-400 mt-8 mb-3">Privacy</h1>
        <p className="text-slate-400 mb-12 text-sm">Last updated {UPDATED}</p>

        <div className="border-l-2 border-emerald-400 pl-6 mb-14">
          <p className="text-lg sm:text-xl text-slate-100 leading-relaxed">
            Verdure runs AI models on your own device. Your conversations, your recordings
            and your files stay there. We do not run a server that sees them, we do not
            collect analytics, and there is no account to create.
          </p>
        </div>

        <Section title="What happens on your device">
          <p>
            The AI model that answers you is downloaded once and then runs entirely on your
            phone or computer. When you send a message, nothing is transmitted — the reply is
            produced locally, which is why Verdure keeps working with no signal at all.
          </p>
          <p>
            The same is true of dictation. When you tap the microphone, audio is turned into
            text by a speech model running on your device. The audio is not uploaded, and it is
            not saved as a recording.
          </p>
          <p>
            Your conversations are stored only on your device, and removing the app removes
            them. Downloaded models are ordinary files you can see and delete from the app&rsquo;s
            Storage screen at any time.
          </p>
        </Section>

        <Section title="Every time Verdure uses the internet">
          <p>This is the complete list.</p>
          <ol className="list-decimal pl-6 space-y-4 marker:text-emerald-400">
            <li>
              <span className="font-semibold text-white">Downloading an AI model.</span>{' '}
              Models are fetched from Hugging Face, a public host we do not operate. Like any
              download, it shows them your IP address. It happens when you choose a model, and
              not again.
            </li>
            <li>
              <span className="font-semibold text-white">
                Checking for updates — desktop only, and only if you turn it on.
              </span>{' '}
              Verdure for Mac can check verdureai.com for a new version. That request would
              reveal your IP address and the fact that you use Verdure, so Verdure asks you
              once and does nothing unless you say yes. You can change the answer, at any time,
              in Storage. On Android and iOS, updates are handled by the app store and Verdure
              never makes this request.
            </li>
          </ol>
          <p>
            There is no third item. Verdure sends no usage statistics, no crash reports, no
            device information and no advertising identifiers, and it contains no third-party
            analytics or advertising code.
          </p>
        </Section>

        <Section title="What we collect">
          <p className="text-xl text-white font-semibold">
            From the app: nothing.
          </p>
          <p>
            Not anonymised, not aggregated, not &ldquo;to improve the service&rdquo;. We have no
            way to know who uses Verdure, how often, or what you ask it. That is a deliberate
            constraint, and it means we have to find out about problems by asking people rather
            than by watching them.
          </p>
        </Section>

        <Section title="Permissions the app asks for">
          <p>
            <span className="font-semibold text-white">Microphone</span> — only used while you
            are actively dictating, and released as soon as you stop. Audio is transcribed on
            your device and discarded. You can refuse it and everything except dictation still
            works.
          </p>
          <p>
            <span className="font-semibold text-white">Network and storage</span> — to download
            AI models and keep them on your device.
          </p>
        </Section>

        <Section title="If you join the waitlist on this website">
          <p>
            The waitlist form is the one place we ask for anything. It collects your first name
            and email address, along with what you told us you want to use AI for and the
            recommendation you were shown. It is stored in a private spreadsheet and used only
            to contact you about Verdure. We do not sell it, share it, or add you to anything
            else.
          </p>
          <p>
            Email us and we will delete it — no explanation required.
          </p>
        </Section>

        <Section title="Children">
          <p>
            Verdure is not directed at children under 13, and since we collect nothing from the
            app, we do not knowingly collect anything from them.
          </p>
        </Section>

        <Section title="Changes to this policy">
          <p>
            If Verdure ever does something new with the network, this page changes and the date
            above changes with it. A policy with an unstated exception is not a policy.
          </p>
        </Section>

        <Section title="Contact">
          <p>
            Questions, deletion requests, or anything else:{' '}
            <a
              href="mailto:glwodu@gmail.com"
              className="text-emerald-400 hover:text-emerald-300 underline"
            >
              glwodu@gmail.com
            </a>
          </p>
        </Section>
      </main>
    </div>
  );
}
