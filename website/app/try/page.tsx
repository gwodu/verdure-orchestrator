import type { Metadata } from 'next';
import Link from 'next/link';
import KeepPosted from '@/components/KeepPosted';
import { MACOS_DMG, RELEASES_PAGE } from '@/lib/links';

export const metadata: Metadata = {
  title: 'Try Verdure on your own device',
  description: 'Verdure the app: a private chat with an AI on your own Mac, iPhone or Android, and dictation into it. Nothing sent anywhere. Download for Mac, or ask for a test build.',
};

// One page, one job: get Verdure onto the reader's own device. The Mac is a
// download; iPhone and Android test builds need us to add the tester by hand,
// so the honest button is "send me one", not a store badge that goes nowhere.
export default function Try() {
  return (
    <main className="max-w-3xl mx-auto px-5 sm:px-8 pt-14 sm:pt-20">
      <h1 className="text-4xl sm:text-5xl font-semibold tracking-tight leading-tight">Try Verdure on your own device.</h1>
      <p className="mt-5 text-lg sm:text-xl text-ink-2 leading-relaxed">
        A private chat with an AI that runs on your device, and dictation straight into it. No account, no
        tracking. Works with no signal at all.
      </p>

      <div className="mt-12 grid gap-4">
        <div className="card p-6 sm:p-8 grid sm:grid-cols-[1fr_auto] gap-5 items-center">
          <div>
            <h2 className="text-xl sm:text-2xl font-semibold tracking-tight mb-2">Mac</h2>
            <p className="text-ink-2">
              Intel and Apple Silicon, macOS 10.15 or later. 8 GB of memory is enough. The app shows you the size
              of the first AI model before it downloads anything.
            </p>
            <p className="text-ink-soft text-sm mt-2">
              Release notes and older versions: <a href={RELEASES_PAGE} className="link" target="_blank" rel="noopener noreferrer">GitHub</a>.
            </p>
          </div>
          <a href={MACOS_DMG} className="btn-primary">Download for Mac</a>
        </div>

        <KeepPosted
          intent="beta build request"
          title="iPhone, iPad or Android"
          blurb="These builds go out through Apple's TestFlight and Google Play's closed testing, which means we add you by hand. Leave the email of the Apple ID or Google account on your phone and we will send the invite."
          button="Send me a test build"
          platforms={[
            { id: 'ios', label: 'iPhone or iPad' },
            { id: 'android', label: 'Android' },
          ]}
        />
      </div>

      <section className="mt-16 text-ink-2 leading-relaxed space-y-4">
        <h2 className="text-2xl font-semibold tracking-tight text-ink">What to expect</h2>
        <p>
          The first time you open it, Verdure picks the most capable AI that fits your device&rsquo;s memory and
          storage and shows you which, with the exact size, before downloading. Downloads resume if they are
          interrupted. It runs on a 2019 Intel MacBook with 8 GB, on a Pixel 8a and on an iPad; the app tells
          you which model it chose and why.
        </p>
        <p>
          Everything you type or say stays on the device. The two things that can reach the internet, the update
          check and an optional web search, are off until you turn them on and are described plainly on the{' '}
          <Link href="/privacy" className="link">privacy page</Link>.
        </p>
        <p className="text-ink-soft">
          Wondering what the computer you already own could run? <Link href="/start" className="link">Ask the recommender</Link>: pick what you want to do and your device, and it tells you.
        </p>
      </section>
    </main>
  );
}
