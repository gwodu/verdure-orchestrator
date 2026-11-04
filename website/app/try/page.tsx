import type { Metadata } from 'next';
import Link from 'next/link';
import KeepPosted from '@/components/KeepPosted';
import QrCode from '@/components/QrCode';
import { MACOS_DMG, PLAY_TEST_URL, RELEASES_PAGE, TESTFLIGHT_URL } from '@/lib/links';

export const metadata: Metadata = {
  title: 'Try Verdure on your own device',
  description: 'Verdure the app: a private chat with an AI on your own Mac, iPhone or Android, and dictation into it. Nothing sent anywhere. Download for Mac, join on TestFlight, or ask for the Android test.',
};

// One page, one job: get Verdure onto the reader's own device. The Mac is a
// download and the iPhone an open TestFlight link, each with a QR code for the
// person reading on a laptop. Android is Google Play internal testing, which
// only works for an account we add by hand, so its honest button is "send me
// an invite", with the Play link underneath for once that is done.
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

        <div className="card p-6 sm:p-8 grid sm:grid-cols-[1fr_auto] gap-5 items-center">
          <div>
            <h2 className="text-xl sm:text-2xl font-semibold tracking-tight mb-2">iPhone and iPad</h2>
            <p className="text-ink-2">
              Through Apple&rsquo;s TestFlight. Open the link on your iPhone or iPad, install TestFlight if it asks,
              then Verdure. No invite needed.
            </p>
            <a href={TESTFLIGHT_URL} className="btn-primary mt-4">Open in TestFlight</a>
          </div>
          <div className="flex sm:flex-col items-center gap-3 text-ink-soft text-xs text-center">
            <QrCode url={TESTFLIGHT_URL} label="QR code for the TestFlight link" />
            <span>Reading this on a laptop?<br />Scan with your phone.</span>
          </div>
        </div>

        <div className="card p-6 sm:p-8">
          <h2 className="text-xl sm:text-2xl font-semibold tracking-tight mb-2">Android</h2>
          <p className="text-ink-2 mb-5">
            Verdure is in Google Play&rsquo;s internal testing, which means we add you by hand first. Leave the
            email of the Google account on your phone and we will add it and reply.
          </p>
          <KeepPosted
            intent="beta build request"
            title=""
            blurb=""
            button="Send me an invite"
            platforms={[{ id: 'android', label: 'Android' }]}
            bare
          />
          <div className="mt-6 pt-6 border-t border-line grid sm:grid-cols-[1fr_auto] gap-5 items-center">
            <div>
              <p className="text-ink-2">Once you have our reply, this opens Verdure on Google Play on your phone.</p>
              <a href={PLAY_TEST_URL} className="btn-text !pl-0">Open on Google Play →</a>
            </div>
            <div className="flex sm:flex-col items-center gap-3 text-ink-soft text-xs text-center">
              <QrCode url={PLAY_TEST_URL} label="QR code for the Google Play test link" />
              <span>Scan with your phone.</span>
            </div>
          </div>
        </div>
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
