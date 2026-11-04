import type { Metadata } from 'next';
import './globals.css';
import SiteHeader from '@/components/SiteHeader';
import SiteFooter from '@/components/SiteFooter';

// System faces only, no webfont download (skills/design.md).
export const metadata: Metadata = {
  title: 'Verdure — run your own AI, offline, on your own device',
  description:
    'Verdure runs AI on the phone or computer you already have. What you say and type stays on it. An app you can use today, and a few lines of code for developers who want it inside their own app.',
  metadataBase: new URL('https://verdureai.com'),
  openGraph: {
    title: 'Verdure — run your own AI, offline, on your own device',
    description: 'AI that runs on the phone or computer you already have. Nothing you say or type leaves it.',
    url: 'https://verdureai.com',
    siteName: 'Verdure',
  },
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body>
        <SiteHeader />
        {children}
        <SiteFooter />
      </body>
    </html>
  );
}
