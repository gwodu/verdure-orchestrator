// The SDK page's facts, kept as data so a test can hold the hello-world
// samples to the API table: every method a sample calls must be a method the
// SDK actually has. Numbers are the measured ones from knowledge/26; when they
// change there, they change here, in one place.

export type ApiRow = { method: string; does: string; fails?: string };

/** Every public method on `Verdure`, as docs/sdk-integration-guide.md §6 lists them. */
export const API: ApiRow[] = [
  { method: 'start(token, policy, dataDir)', does: 'Verify the licence offline, read the device. No network, instant.', fails: 'License' },
  { method: 'capabilities()', does: 'Tier, free memory and disk, cores, GPU class, engines built, model attributions, notes.' },
  { method: 'loadManifest(json, signature, publicKey)', does: 'Trust a signed manifest you ship inside the app — the fully offline path.', fails: 'Integrity' },
  { method: 'setBundleSource(url, publicKey)', does: 'Fetch and verify a signed manifest from your host instead.', fails: 'Policy, Network, Integrity' },
  { method: 'recommend(task)', does: 'The bundle that fits this device now and keeps pace with live speech where we measured it. Says what it skipped and why.', fails: 'NoBundle' },
  { method: 'recommendFor(task, pace)', does: 'The same with the pace stated: Pace.RECORDED allows a slower, better bundle for audio already captured.', fails: 'NoBundle' },
  { method: 'fit(bundleId)', does: 'Judge a bundle you chose yourself: fits, plus every problem as a sentence. Nothing is substituted.', fails: 'NoBundle' },
  { method: 'fitFor(bundleId, pace)', does: 'The same for the pace stated.', fails: 'NoBundle' },
  { method: 'download(bundleId, listener)', does: 'Download or resume every file, verified by hash, reusing any model another bundle already holds. Bytes and one sentence per file to the listener.', fails: 'License, Policy, Network, Integrity, Cancelled' },
  { method: 'isDownloaded(bundleId)', does: 'All files present and their hashes match.' },
  { method: 'load(bundleId)', does: 'Load the engines. Refuses with the numbers when memory is short. 1.2 s for the base bundle, 15 s for the small one on a Pixel 8a.', fails: 'Memory, EngineUnavailable, Engine' },
  { method: 'loadOvercommit(bundleId)', does: 'load without the memory gate, for a device you have measured yourself.', fails: 'as load' },
  { method: 'transcribe(config)', does: 'Open a stream: pushPcm(samples, rate) returns transcript events; finish() flushes.', fails: 'EngineUnavailable' },
  { method: 'extract(text, schemaJson)', does: 'JSON valid against your schema, or an error naming each field that was wrong. Blocks for the whole generation.', fails: 'EngineUnavailable, InvalidSchema, SchemaInvalid' },
  { method: 'chat(messages, sink)', does: 'One conversation turn, streamed token by token.', fails: 'EngineUnavailable, Engine' },
  { method: 'index(collection, docs)', does: 'Chunk and index documents already on the device.', fails: 'Storage' },
  { method: 'query(collection, question, k)', does: 'The best k passages.', fails: 'Storage' },
  { method: 'delete(scope)', does: 'A collection, the indexes, or everything — the encryption key first, then the files.', fails: 'Storage' },
  { method: 'setCloudFallback(hook)', does: 'Give Verdure your cloud for when no engine is loaded. Verdure never opens that socket itself.' },
];

export type Sample = { id: 'kotlin' | 'rust' | 'python'; label: string; code: string };

export const SAMPLES: Sample[] = [
  {
    id: 'kotlin',
    label: 'Kotlin (Android)',
    code: `import uniffi.verdure.*

val sdk = Verdure.start(token, PolicyConfig(networkDownloads = true, allowedHosts = emptyList(),
                        localCounters = false, encryptedStorage = true, retentionDays = null),
                        filesDir.resolve("verdure").absolutePath)
sdk.loadManifest(manifestJson, signature, publicKey)

val bundle = sdk.recommend(Task.SCRIBE)        // what this phone can run live; bundle.skipped says why not the rest
if (!sdk.isDownloaded(bundle.id)) sdk.download(bundle.id, listener)
sdk.load(bundle.id)

val stream = sdk.transcribe(AsrConfig(langHints = listOf("en"), keyterms = listOf(), replacements = listOf()))
for (ev in stream.pushPcm(floats, 16000u))     // 16 kHz mono from AudioRecord, as floats in [-1, 1]
    if (ev is TranscriptEvent.Final) show(ev.text)

val facts = sdk.extract(transcript, schemaJson).json   // JSON that validates against YOUR schema`,
  },
  {
    id: 'rust',
    label: 'Rust',
    code: `use verdure::{AsrConfig, PolicyConfig, Task, TranscriptEvent, Verdure};

let sdk = Verdure::start(token, PolicyConfig::default(), data_dir)?;
sdk.load_manifest(manifest_json, signature, public_key)?;

let bundle = sdk.recommend(Task::Scribe)?;     // or recommend_for(Task::Scribe, Pace::Recorded)
if !sdk.is_downloaded(bundle.id.clone()) { sdk.download(bundle.id.clone(), listener)?; }
sdk.load(bundle.id)?;

let stream = sdk.transcribe(AsrConfig { lang_hints: vec!["en".into()], keyterms: vec![], replacements: vec![] })?;
for ev in stream.push_pcm(samples, 16_000)? {
    if let TranscriptEvent::Final { text, .. } = ev { println!("{text}"); }
}

let out = sdk.extract(transcript, schema_json)?;   // out.json validates against your schema`,
  },
  {
    id: 'python',
    label: 'Python',
    code: `import verdure as v

sdk = v.Verdure.start(token, v.PolicyConfig(network_downloads=True, allowed_hosts=[],
                      local_counters=False, encrypted_storage=True, retention_days=None), data_dir)
sdk.load_manifest(manifest_json, signature, public_key)

bundle = sdk.recommend(v.Task.SCRIBE)
if not sdk.is_downloaded(bundle.id):
    sdk.download(bundle.id, listener)
sdk.load(bundle.id)

stream = sdk.transcribe(v.AsrConfig(lang_hints=["en"], keyterms=[], replacements=[]))
for ev in stream.push_pcm(samples, 16000):
    if isinstance(ev, v.TranscriptEvent.FINAL): print(ev.text)

out = sdk.extract(transcript, schema_json)   # out.json validates against your schema`,
  },
];

/** Device tiers, knowledge/02 names. */
export const TIERS = [
  { tier: 'P1', device: '4 GB phone', example: 'Itel A70, Tecno Spark Go' },
  { tier: 'P2', device: '6 GB phone', example: 'Redmi 13C, Galaxy A16' },
  { tier: 'P3', device: '8 GB phone', example: 'Pixel 8a, iPhone 12 and later' },
  { tier: 'L1', device: '4 GB laptop', example: 'ThinkPad T460' },
  { tier: 'L2', device: '8 GB laptop', example: '2019 Intel MacBook, a school-lab PC' },
  { tier: 'L3', device: '16 GB+ laptop', example: 'Apple Silicon, or a discrete GPU' },
];

/**
 * The speech bake-off and the phone run (knowledge/26, "The speech bake-off",
 * "Small on the Pixel"): ten real two-speaker GP consultations (PriMock57),
 * scored on word error rate and on a list of clinical terms.
 */
export const SPEECH = [
  { model: 'Whisper base', bundle: 'scribe-floor', download: '148 MB', wer: '26.5 %', missed: '1 in 3.6', mac: '0.28×', pixel: '1.25×' },
  { model: 'Whisper small', bundle: 'scribe-small', download: '488 MB', wer: '21.6 %', missed: '1 in 6', mac: '0.75×', pixel: '3.2×' },
];

/** Model and engine licences in the shipped bundles (knowledge/09, 26). */
export const LICENCES = [
  { what: 'Whisper (base, small) weights', licence: 'MIT', from: 'OpenAI' },
  { what: 'whisper.cpp, llama.cpp (the engines)', licence: 'MIT', from: 'ggml' },
  { what: 'Qwen2.5 0.5B and 1.5B Instruct', licence: 'Apache-2.0', from: 'Alibaba Qwen' },
];
