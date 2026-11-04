import Link from 'next/link';
import { CONTACT_EMAIL } from '@/lib/links';

export default function SiteFooter() {
  return (
    <footer className="mt-28">
      <div className="max-w-5xl mx-auto px-5 sm:px-8 py-10 flex flex-col sm:flex-row sm:items-center justify-between gap-4 text-sm text-ink-soft">
        <p>© {new Date().getFullYear()} Verdure. AI that runs on your own device.</p>
        <nav className="flex flex-wrap gap-x-6 gap-y-2">
          <Link href="/developers" className="hover:text-ink">For developers</Link>
          <Link href="/try" className="hover:text-ink">Try the app</Link>
          <Link href="/privacy" className="hover:text-ink">Privacy</Link>
          <a href={`mailto:${CONTACT_EMAIL}`} className="hover:text-ink">{CONTACT_EMAIL}</a>
        </nav>
      </div>
    </footer>
  );
}
