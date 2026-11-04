import Link from 'next/link';

// One header on every page: the name, the developers page, and the one
// action — try the app. Nothing else competes with it.
export default function SiteHeader() {
  return (
    <header className="sticky top-0 z-20 bg-paper/90 backdrop-blur-sm">
      <div className="max-w-5xl mx-auto px-5 sm:px-8 h-16 flex items-center justify-between gap-6">
        <Link href="/" className="text-lg font-semibold tracking-tight text-ink" aria-label="Verdure home">
          Verdure
        </Link>
        <nav className="flex items-center gap-5 sm:gap-8 text-[15px] whitespace-nowrap">
          <Link href="/developers" className="text-ink-2 hover:text-ink">For developers</Link>
          <Link href="/try" className="btn-primary !py-2 !px-3.5 text-sm">Try the app</Link>
        </nav>
      </div>
    </header>
  );
}
