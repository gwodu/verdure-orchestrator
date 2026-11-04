import type { Metadata } from 'next';
import './globals.css';
import SiteHeader from '@/components/SiteHeader';
import SiteFooter from '@/components/SiteFooter';

// System faces only, no webfont download — the identity in skills/design.md.
export const metadata: Metadata = {
  title: 'Verdure — run local models with a few lines of code',
  description:
    'Verdure is a runtime that turns speech into text and text into structured JSON on the user’s own phone or laptop, offline. One library, one licence token, nothing leaves the device.',
  metadataBase: new URL('https://verdureai.com'),
  openGraph: {
    title: 'Verdure — run local models with a few lines of code',
    description: 'Speech to transcript to schema-valid JSON, on the device in front of you. Offline. Private.',
    url: 'https://verdureai.com',
    siteName: 'Verdure',
  },
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body className="antialiased">
        <SiteHeader />
        {children}
        <SiteFooter />
      </body>
    </html>
  );
}
