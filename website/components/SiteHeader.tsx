import Link from 'next/link';

// One header on every page: the wordmark home, three destinations. The site
// has one job per page (skills/copywriting.md), so the nav names the pages,
// not features.
export default function SiteHeader() {
  return (
    <header className="sticky top-0 z-20 backdrop-blur-md bg-[rgba(2,19,41,0.7)] border-b border-[rgba(76,216,173,0.15)]">
      <div className="max-w-6xl mx-auto px-5 sm:px-8 h-16 flex items-center justify-between gap-6">
        <Link href="/" className="wordmark text-lg sm:text-xl font-semibold" aria-label="Verdure home">
          Verdure
        </Link>
        <nav className="flex items-center gap-4 sm:gap-8 text-sm sm:text-base whitespace-nowrap">
          <Link href="/sdk" className="text-ink hover:text-mint-bright transition-colors">SDK</Link>
          <Link href="/try" className="text-ink hover:text-mint-bright transition-colors">Try the app</Link>
          <Link href="/privacy" className="text-ink-soft hover:text-mint-bright transition-colors hidden sm:inline">Privacy</Link>
          <a href="/sdk#hello" className="btn-primary !py-1.5 !px-3.5 text-sm hidden sm:inline-flex">Hello world</a>
        </nav>
      </div>
    </header>
  );
}
