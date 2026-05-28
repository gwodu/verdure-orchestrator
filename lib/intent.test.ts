import { describe, it, expect } from 'vitest';
import { classifyIntent } from './intent';

describe('classifyIntent', () => {
  it('detects creative intent', () => {
    expect(classifyIntent('I want to write fantasy novels')).toBe('creative');
    expect(classifyIntent('help me brainstorm blog post ideas')).toBe('creative');
    expect(classifyIntent('Write me a short poem')).toBe('creative');
  });

  it('detects code intent', () => {
    expect(classifyIntent('help me debug Rust code at work')).toBe('code');
    expect(classifyIntent('explain this Python function')).toBe('code');
    expect(classifyIntent('I need help with SQL queries')).toBe('code');
  });

  it('detects multilingual intent', () => {
    expect(classifyIntent('my kid needs Spanish homework help')).toBe('multilingual');
    expect(classifyIntent('translate this from French')).toBe('multilingual');
    expect(classifyIntent('I want to practice Mandarin')).toBe('multilingual');
  });

  it('falls back to general for plain chat', () => {
    expect(classifyIntent('I want to ask it questions')).toBe('general');
    expect(classifyIntent('summarize meeting notes')).toBe('general');
    expect(classifyIntent('')).toBe('general');
  });

  it('is case-insensitive', () => {
    expect(classifyIntent('WRITE A STORY')).toBe('creative');
    expect(classifyIntent('Debug My Code')).toBe('code');
  });

  it('priority order: code wins over creative when both keywords are present', () => {
    // "write" is creative, "code" is code. PRIORITY=['code', 'creative', 'multilingual'].
    expect(classifyIntent('write code for me')).toBe('code');
  });
});
