import Link from 'next/link';
import { CONTACT_EMAIL, LINKEDIN, RELEASES_PAGE } from '@/lib/links';

export default function SiteFooter() {
  return (
    <footer className="mt-24 border-t border-[rgba(76,216,173,0.15)]">
      <div className="max-w-6xl mx-auto px-5 sm:px-8 py-10 flex flex-col sm:flex-row sm:items-center justify-between gap-6 text-sm text-ink-soft">
        <div>
          <p className="wordmark text-base font-semibold mb-1">Verdure</p>
          <p>Local AI that runs on the device in front of you. © {new Date().getFullYear()} Verdure.</p>
        </div>
        <nav className="flex flex-wrap gap-x-6 gap-y-2">
          <Link href="/sdk" className="hover:text-mint-bright">SDK</Link>
          <Link href="/try" className="hover:text-mint-bright">Try the app</Link>
          <a href={RELEASES_PAGE} className="hover:text-mint-bright" target="_blank" rel="noopener noreferrer">Releases</a>
          <Link href="/privacy" className="hover:text-mint-bright">Privacy</Link>
          <a href={`mailto:${CONTACT_EMAIL}`} className="hover:text-mint-bright">{CONTACT_EMAIL}</a>
          <a href={LINKEDIN} className="hover:text-mint-bright" target="_blank" rel="noopener noreferrer">LinkedIn</a>
        </nav>
      </div>
    </footer>
  );
}
