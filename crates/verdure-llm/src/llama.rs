//! llama.cpp as an `LlmEngine`. Weights resident for the engine's lifetime;
//! a fresh context per request (knowledge/22 — the KV cache across turns is
//! a measured-first refinement). Sampling, backend choice and the one-decoder
//! rule are lifted from the app's `llm.rs`, each with the bug it fixed.
//!
//! **Grammar sampling is opt-in** (`LoadOptions::allow_grammar`). The app
//! removed llama.cpp's grammar sampler after it `ggml_abort()`ed the process
//! on a multi-byte token (knowledge/18); an abort kills the vendor's app. Until
//! the engine runs isolated (`[OQ-58]`, `[OQ-108]`) a request carrying a
//! grammar is refused with `EngineUnavailable` rather than risked.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::OnceLock;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use verdure_core::{Control, GenRequest, LlmEngine, VError};

use crate::context::{fit_messages, max_prompt_tokens};
use crate::prompt::{format_chat, PromptStyle};

const N_BATCH: u32 = 512;
const SEED: u32 = 0x5EED;

static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();

fn backend() -> Result<&'static LlamaBackend, VError> {
    if let Some(b) = BACKEND.get() {
        return Ok(b);
    }
    let b = LlamaBackend::init()
        .map_err(|e| VError::Engine(format!("couldn't start the engine: {e}")))?;
    Ok(BACKEND.get_or_init(|| b))
}

#[derive(Debug, Clone)]
pub struct LoadOptions {
    pub style: PromptStyle,
    pub n_ctx: u32,
    pub threads: u32,
    /// Keep every layer on the CPU and hide GPU devices. Forced on x86_64
    /// macOS where Metal returns wrong numbers (`[OQ-71]`).
    pub cpu_only: bool,
    pub allow_grammar: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        LoadOptions {
            style: PromptStyle::ChatMl,
            n_ctx: 2048,
            threads: 4,
            cpu_only: false,
            allow_grammar: false,
        }
    }
}

pub struct LlamaEngine {
    id: String,
    model: LlamaModel,
    opts: LoadOptions,
    /// One generation at a time. Two concurrent `generate` calls would each
    /// build a context (hundreds of MB on a 4B) and compete for every core;
    /// queuing them is the honest behaviour on a phone. Transcription runs on
    /// a different engine and is not held up by this lock.
    busy: std::sync::Mutex<()>,
}

impl LlamaEngine {
    pub fn load(path: &Path, opts: LoadOptions) -> Result<LlamaEngine, VError> {
        let mut params = LlamaModelParams::default();
        let cpu_only = opts.cpu_only || cfg!(all(target_os = "macos", target_arch = "x86_64"));
        if cpu_only {
            params = params.with_n_gpu_layers(0);
            let cpu: Vec<usize> = llama_cpp_2::list_llama_ggml_backend_devices()
                .into_iter()
                .filter(|d| d.backend.eq_ignore_ascii_case("CPU"))
                .map(|d| d.index)
                .collect();
            params = params
                .with_devices(&cpu)
                .unwrap_or_else(|_| LlamaModelParams::default().with_n_gpu_layers(0));
        }
        let model = LlamaModel::load_from_file(backend()?, path, &params)
            .map_err(|e| VError::Engine(format!("couldn't open the model: {e}")))?;
        let id = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| "llama".into());
        Ok(LlamaEngine {
            id,
            model,
            opts,
            busy: std::sync::Mutex::new(()),
        })
    }
}

impl LlmEngine for LlamaEngine {
    fn id(&self) -> &str {
        &self.id
    }

    fn context_window(&self) -> u32 {
        self.opts.n_ctx
    }

    fn count_tokens(&self, text: &str) -> usize {
        self.model
            .str_to_token(text, AddBos::Never)
            .map(|t| t.len())
            .unwrap_or(text.len() / 3 + 1)
    }

    fn generate(
        &self,
        req: &GenRequest,
        on_token: &mut dyn FnMut(&str) -> Control,
    ) -> Result<String, VError> {
        let _one_at_a_time = self.busy.lock().unwrap_or_else(|e| e.into_inner());
        if req.grammar.is_some() && !self.opts.allow_grammar {
            return Err(VError::EngineUnavailable(
                "grammar-constrained sampling is switched off in this build (it can abort the host process; see [OQ-108])".into(),
            ));
        }
        let n_ctx = self.opts.n_ctx.max(crate::context::MIN_CONTEXT);
        let max_new = req.max_tokens.min(n_ctx / 2).max(16) as i32;
        let cost = |s: &str| self.count_tokens(s);
        let kept = fit_messages(&req.messages, n_ctx, max_new as usize + 48, &cost);
        let kept: Vec<verdure_core::Msg> = kept.into_iter().cloned().collect();
        let prompt = format_chat(&kept, self.opts.style);
        let add_bos = if self.opts.style.add_bos() {
            AddBos::Always
        } else {
            AddBos::Never
        };
        let mut tokens = self
            .model
            .str_to_token(&prompt, add_bos)
            .map_err(|e| VError::Engine(format!("tokenise: {e}")))?;
        let cap = max_prompt_tokens(n_ctx);
        if tokens.len() > cap {
            tokens = tokens.split_off(tokens.len() - cap);
        }

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(n_ctx))
            .with_n_batch(N_BATCH)
            .with_n_threads(self.opts.threads.max(1) as i32)
            .with_n_threads_batch(self.opts.threads.max(1) as i32);
        let mut ctx = self
            .model
            .new_context(backend()?, ctx_params)
            .map_err(|e| VError::Engine(format!("context: {e}")))?;

        let mut batch = LlamaBatch::new(N_BATCH as usize, 1);
        let last_idx = tokens.len().saturating_sub(1);
        let mut pos: i32 = 0;
        for chunk in tokens.chunks(N_BATCH as usize) {
            batch.clear();
            for token in chunk {
                batch
                    .add(*token, pos, &[0], pos as usize == last_idx)
                    .map_err(|e| VError::Engine(e.to_string()))?;
                pos += 1;
            }
            ctx.decode(&mut batch)
                .map_err(|e| VError::Engine(e.to_string()))?;
        }

        let mut chain = vec![LlamaSampler::penalties(64, 1.1, 0.0, 0.0)];
        if let Some(g) = &req.grammar {
            chain.push(LlamaSampler::grammar(&self.model, g, "root").map_err(|e| {
                VError::InvalidSchema(format!("grammar rejected by the engine: {e:?}"))
            })?);
        }
        if req.temperature <= 0.0 {
            chain.push(LlamaSampler::greedy());
        } else {
            chain.extend([
                LlamaSampler::top_k(40),
                LlamaSampler::top_p(0.9, 1),
                LlamaSampler::temp(req.temperature),
                LlamaSampler::dist(SEED),
            ]);
        }
        let mut sampler = LlamaSampler::chain_simple(chain);

        let mut reply = String::new();
        let mut utf8 = encoding_rs::UTF_8.new_decoder();
        let mut n_cur = pos;
        for _ in 0..max_new {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(token);
            if self.model.is_eog_token(token) {
                break;
            }
            if let Ok(piece) = self.model.token_to_piece(token, &mut utf8, true, None) {
                reply.push_str(&piece);
                if on_token(&piece) == Control::Stop {
                    break;
                }
            }
            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|e| VError::Engine(e.to_string()))?;
            n_cur += 1;
            ctx.decode(&mut batch)
                .map_err(|e| VError::Engine(e.to_string()))?;
            if n_cur >= n_ctx as i32 {
                break;
            }
        }
        Ok(reply.trim().to_string())
    }
}
