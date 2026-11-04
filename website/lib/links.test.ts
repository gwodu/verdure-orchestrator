import { describe, expect, it } from 'vitest';
import { CONTACT_EMAIL, CONTACT_MAILTO, MACOS_DMG, PLAY_TEST_URL, RELEASES_PAGE, SDK_REQUEST_MAILTO, TESTFLIGHT_URL, mailto } from './links';

describe('the links the site hands out', () => {
  it('points downloads at the public releases repository, never at the private source or the updater endpoint', () => {
    expect(RELEASES_PAGE).toMatch(/^https:\/\/github\.com\/gwodu\/Verdure-Releases\/releases\/latest$/);
    expect(MACOS_DMG.startsWith(RELEASES_PAGE + '/download/')).toBe(true);
    expect(MACOS_DMG).toMatch(/\.dmg$/);
    expect(MACOS_DMG).not.toContain('verdureai.com/updates');
  });

  it('hands out the phone test builds where the stores host them', () => {
    expect(TESTFLIGHT_URL).toMatch(/^https:\/\/testflight\.apple\.com\/join\/[A-Za-z0-9]+$/);
    expect(PLAY_TEST_URL).toMatch(/^https:\/\/play\.google\.com\/apps\/internaltest\/\d+$/);
  });

  it('reaches the founder at the work address, not a personal one (founder, 2026-11-04)', () => {
    expect(CONTACT_EMAIL).toBe('wodu@verdureai.com');
    expect(CONTACT_MAILTO.startsWith('mailto:wodu@verdureai.com?')).toBe(true);
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
