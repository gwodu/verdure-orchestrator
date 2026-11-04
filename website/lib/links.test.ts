import { describe, expect, it } from 'vitest';
import { CONTACT_EMAIL, MACOS_DMG, RELEASES_PAGE, SDK_REQUEST_MAILTO, mailto } from './links';

describe('the links the site hands out', () => {
  it('points downloads at the public releases repository, never at the private source or the updater endpoint', () => {
    expect(RELEASES_PAGE).toMatch(/^https:\/\/github\.com\/gwodu\/Verdure-Releases\/releases\/latest$/);
    expect(MACOS_DMG.startsWith(RELEASES_PAGE + '/download/')).toBe(true);
    expect(MACOS_DMG).toMatch(/\.dmg$/);
    expect(MACOS_DMG).not.toContain('verdureai.com/updates');
  });

  it('builds a mailto that mail clients decode, spaces and newlines included', () => {
    const m = mailto('A subject with spaces', 'line one\nline two');
    expect(m.startsWith(`mailto:${CONTACT_EMAIL}?`)).toBe(true);
    expect(m).toContain('subject=A%20subject%20with%20spaces');
    expect(m).toContain('body=line%20one%0Aline%20two');
    expect(m).not.toContain('+');
    expect(SDK_REQUEST_MAILTO).toContain('subject=Verdure%20SDK');
  });
});
