import { describe, it, expect } from 'vitest';
import {
  WAITLIST_COLUMNS,
  buildWaitlistRow,
  loadSheetsConfig,
  isSheetsConfigured,
  normalizePrivateKey,
  type WaitlistSignup,
} from './sheets';

const SIGNUP: WaitlistSignup = {
  firstName: 'Ada',
  email: 'ada@example.com',
  intentText: 'I want to write fantasy novels',
  modelName: 'Gemma 3 4B',
  tier: 'L2',
};

const FULL_ENV = {
  GOOGLE_SHEETS_CLIENT_EMAIL: 'bot@project.iam.gserviceaccount.com',
  GOOGLE_SHEETS_PRIVATE_KEY: '-----BEGIN PRIVATE KEY-----\\nabc\\ndef\\n-----END PRIVATE KEY-----\\n',
  WAITLIST_SHEET_ID: 'sheet-123',
};

describe('normalizePrivateKey', () => {
  it('turns escaped \\n sequences into real newlines', () => {
    expect(normalizePrivateKey('line1\\nline2')).toBe('line1\nline2');
  });

  it('leaves already-real newlines untouched', () => {
    expect(normalizePrivateKey('line1\nline2')).toBe('line1\nline2');
  });
});

describe('buildWaitlistRow', () => {
  it('emits values in the documented column order', () => {
    const row = buildWaitlistRow(SIGNUP, '2026-05-29T12:00:00.000Z');
    expect(row).toEqual([
      '2026-05-29T12:00:00.000Z',
      'Ada',
      'ada@example.com',
      'I want to write fantasy novels',
      'Gemma 3 4B',
      'L2',
    ]);
  });

  it('produces one cell per declared column', () => {
    const row = buildWaitlistRow(SIGNUP, '2026-05-29T12:00:00.000Z');
    expect(row).toHaveLength(WAITLIST_COLUMNS.length);
  });
});

describe('loadSheetsConfig', () => {
  it('returns null when any required var is missing', () => {
    expect(loadSheetsConfig({})).toBeNull();
    expect(loadSheetsConfig({ ...FULL_ENV, WAITLIST_SHEET_ID: undefined })).toBeNull();
    expect(loadSheetsConfig({ ...FULL_ENV, GOOGLE_SHEETS_CLIENT_EMAIL: undefined })).toBeNull();
    expect(loadSheetsConfig({ ...FULL_ENV, GOOGLE_SHEETS_PRIVATE_KEY: undefined })).toBeNull();
  });

  it('treats blank/whitespace values as missing', () => {
    expect(loadSheetsConfig({ ...FULL_ENV, WAITLIST_SHEET_ID: '   ' })).toBeNull();
  });

  it('resolves config and normalizes the private key when fully set', () => {
    const config = loadSheetsConfig(FULL_ENV);
    expect(config).not.toBeNull();
    expect(config?.clientEmail).toBe('bot@project.iam.gserviceaccount.com');
    expect(config?.spreadsheetId).toBe('sheet-123');
    expect(config?.privateKey).toContain('\n');
    expect(config?.privateKey).not.toContain('\\n');
  });

  it('defaults the range but honors an override', () => {
    expect(loadSheetsConfig(FULL_ENV)?.range).toBe('Sheet1!A:F');
    expect(
      loadSheetsConfig({ ...FULL_ENV, WAITLIST_SHEET_RANGE: 'Signups!A:F' })?.range,
    ).toBe('Signups!A:F');
  });
});

describe('isSheetsConfigured', () => {
  it('is true only when the full config resolves', () => {
    expect(isSheetsConfigured(FULL_ENV)).toBe(true);
    expect(isSheetsConfigured({})).toBe(false);
  });
});
