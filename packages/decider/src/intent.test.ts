import { describe, it, expect } from 'vitest';
import { classifyIntent } from './intent';

describe('classifyIntent — text', () => {
  it('detects creative intent', () => {
    expect(classifyIntent('I want to write fantasy novels')).toEqual({ modality: 'text', task: 'creative' });
    expect(classifyIntent('help me brainstorm blog post ideas')).toEqual({ modality: 'text', task: 'creative' });
  });

  it('detects code intent', () => {
    expect(classifyIntent('help me debug Rust code at work')).toEqual({ modality: 'text', task: 'code' });
    expect(classifyIntent('explain this Python function')).toEqual({ modality: 'text', task: 'code' });
  });

  it('detects multilingual intent', () => {
    expect(classifyIntent('translate this from French')).toEqual({ modality: 'text', task: 'multilingual' });
    expect(classifyIntent('my kid needs Spanish homework help')).toEqual({ modality: 'text', task: 'multilingual' });
  });

  it('falls back to general text for unrecognized intent', () => {
    expect(classifyIntent('help me think through a decision')).toEqual({ modality: 'text', task: 'general' });
    expect(classifyIntent('')).toEqual({ modality: 'text', task: 'general' });
  });
});

describe('classifyIntent — audio', () => {
  it('detects transcription', () => {
    expect(classifyIntent('transcribe this interview')).toEqual({ modality: 'audio', task: 'transcription' });
    expect(classifyIntent('I need subtitles for my video file')).toEqual({ modality: 'audio', task: 'transcription' });
  });

  it('detects text-to-speech', () => {
    expect(classifyIntent('turn my article into a voiceover')).toEqual({ modality: 'audio', task: 'speech' });
    expect(classifyIntent('read this aloud as an audiobook')).toEqual({ modality: 'audio', task: 'speech' });
  });

  it('detects stem separation', () => {
    expect(classifyIntent('remove vocals from this song')).toEqual({ modality: 'audio', task: 'separation' });
    expect(classifyIntent('make a karaoke track')).toEqual({ modality: 'audio', task: 'separation' });
  });

  it('detects music generation', () => {
    expect(classifyIntent('compose music for my game')).toEqual({ modality: 'audio', task: 'music' });
    expect(classifyIntent('I need background music')).toEqual({ modality: 'audio', task: 'music' });
  });

  it('a bare recording defaults to transcription (the useful, feasible default)', () => {
    expect(classifyIntent('I have an audio recording from a meeting')).toEqual({ modality: 'audio', task: 'transcription' });
  });

  it('music wins over the creative "write" keyword', () => {
    // "write music" should be audio, not text/creative.
    expect(classifyIntent('help me make music')).toEqual({ modality: 'audio', task: 'music' });
  });
});

describe('classifyIntent — image', () => {
  it('detects generation', () => {
    expect(classifyIntent('create a logo for my shop')).toEqual({ modality: 'image', task: 'generate' });
    expect(classifyIntent('I want a picture of a sunset')).toEqual({ modality: 'image', task: 'generate' });
  });

  it('detects editing', () => {
    expect(classifyIntent('remove the background from my photo')).toEqual({ modality: 'image', task: 'edit' });
  });

  it('detects upscaling', () => {
    expect(classifyIntent('upscale this old picture')).toEqual({ modality: 'image', task: 'upscale' });
    expect(classifyIntent('make my photo sharper and bigger')).toEqual({ modality: 'image', task: 'upscale' });
  });
});

describe('classifyIntent — video', () => {
  it('detects generation', () => {
    expect(classifyIntent('make a video of a dog surfing')).toEqual({ modality: 'video', task: 'generate' });
    expect(classifyIntent('create an animation')).toEqual({ modality: 'video', task: 'generate' });
  });

  it('detects enhancement', () => {
    expect(classifyIntent('make my video smoother with slow motion')).toEqual({ modality: 'video', task: 'enhance' });
  });

  // Substring matching misread these (2026-10-20 inspection probe). Each now
  // falls through to the general text default instead of a wrong specialist.
  it.each([
    ['explain how the immune system works'],
    ['learn about ecosystems'],
    ['what should I go to the store for'],
    ['fix my resume'],
    ['photosynthesis homework'],
    ['what is the function of the liver'],
    ['a good TV program to watch'],
  ])('does not misread %j', (text) => {
    expect(classifyIntent(text)).toEqual({ modality: 'text', task: 'general' });
  });

  it('still matches whole words, phrases, plurals and symbols', () => {
    expect(classifyIntent('create images from a description')).toEqual({ modality: 'image', task: 'generate' });
    expect(classifyIntent('turn my recordings into text')).toEqual({ modality: 'audio', task: 'transcription' });
    expect(classifyIntent('help me with c++ templates')).toEqual({ modality: 'text', task: 'code' });
    expect(classifyIntent('fix this python bug')).toEqual({ modality: 'text', task: 'code' });
    expect(classifyIntent('speech to text for lectures')).toEqual({ modality: 'audio', task: 'transcription' });
  });
});
