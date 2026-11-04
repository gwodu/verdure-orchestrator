// Every outward link the site shows, in one place, so a moved download or a
// changed address is one edit — and a test can check they still point where
// knowledge/21 ("Distribution") says they do.

/** Public downloads-only repository (no source) — knowledge/21, "Distribution". */
export const RELEASES_PAGE = 'https://github.com/gwodu/Verdure-Releases/releases/latest';

/** The macOS disk image, straight from the latest release. Universal (Intel + Apple Silicon). */
export const MACOS_DMG = `${RELEASES_PAGE}/download/Verdure_0.1.0_universal.dmg`;

/** The founder's address on the company domain (founder, 2026-11-04). */
export const CONTACT_EMAIL = 'wodu@verdureai.com';

export const LINKEDIN = 'https://www.linkedin.com/in/gesikeme-wodu';

/** A mailto with the subject and body filled in, safe for any characters. */
export function mailto(subject: string, body = ''): string {
  const q = new URLSearchParams();
  q.set('subject', subject);
  if (body) q.set('body', body);
  // URLSearchParams encodes spaces as '+', which mail clients do not decode.
  return `mailto:${CONTACT_EMAIL}?${q.toString().replace(/\+/g, '%20')}`;
}

export const SDK_REQUEST_MAILTO = mailto(
  'Verdure SDK: send me the test kit',
  'Hi Gesi,\n\nI would like to try the Verdure SDK.\n\nPlatform (Android / macOS / Linux / Windows / Python):\nWhat we are building:\n',
);

export const CONTACT_MAILTO = mailto('Verdure', 'Hi Gesi,\n\n');
