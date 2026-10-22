//! Sizing the context window and trimming a conversation to fit — pure
//! arithmetic over the KV-cache cost, lifted from the app (knowledge/22).

use verdure_core::{Msg, Role};

/// Below this a window isn't a degraded chat, it's a broken one.
pub const MIN_CONTEXT: u32 = 512;
/// Capped by TIME, not memory: prefill is linear in tokens on a CPU.
pub const MAX_CONTEXT: u32 = 4096;
const MAX_NEW_TOKENS: u32 = 1024;
const TURN_MARKUP_TOKENS: usize = 12;

/// How large a window `spare_mb` buys at `kv_kb_per_token`, clamped to
/// `[MIN_CONTEXT, min(MAX_CONTEXT, trained_ctx)]`.
pub fn context_for_budget(spare_mb: u64, kv_kb_per_token: u32, trained_ctx: u32) -> u32 {
    let per_token = kv_kb_per_token.max(1) as u64;
    let affordable = spare_mb.saturating_mul(1024) / per_token;
    let ceiling = trained_ctx.max(MIN_CONTEXT).min(MAX_CONTEXT);
    (affordable.min(ceiling as u64) as u32)
        .max(MIN_CONTEXT)
        .min(ceiling)
}

/// Tokens to reserve for the reply: half the window, within bounds.
pub fn reply_budget(n_ctx: u32) -> u32 {
    (n_ctx / 2).min(MAX_NEW_TOKENS).max(128)
}

/// Prompt tokens that still leave room to answer. Never 0.
pub fn max_prompt_tokens(n_ctx: u32) -> usize {
    (n_ctx as usize)
        .saturating_sub(reply_budget(n_ctx) as usize + 8)
        .max(16)
}

/// The suffix of `messages` that fits: system messages always, then whole
/// user/assistant turns newest-first until the budget is spent. The final
/// user message is always kept. `cost` is the model's own tokenizer.
pub fn fit_messages<'a>(
    messages: &'a [Msg],
    n_ctx: u32,
    reserved: usize,
    cost: &dyn Fn(&str) -> usize,
) -> Vec<&'a Msg> {
    let system: Vec<&Msg> = messages.iter().filter(|m| m.role == Role::System).collect();
    let rest: Vec<&Msg> = messages.iter().filter(|m| m.role != Role::System).collect();
    let budget = n_ctx as usize;
    let mut used = reserved
        + system
            .iter()
            .map(|m| cost(&m.content) + TURN_MARKUP_TOKENS)
            .sum::<usize>();
    let mut keep_from = rest.len();
    // Always keep the last message (the question being asked).
    if let Some(last) = rest.last() {
        used += cost(&last.content) + TURN_MARKUP_TOKENS;
        keep_from = rest.len() - 1;
    }
    // Then whole turns (user + assistant pairs) newest first.
    let mut i = keep_from;
    while i >= 2 {
        let turn_cost =
            cost(&rest[i - 1].content) + cost(&rest[i - 2].content) + 2 * TURN_MARKUP_TOKENS;
        if used + turn_cost > budget {
            break;
        }
        used += turn_cost;
        i -= 2;
    }
    let mut out = system;
    out.extend_from_slice(&rest[i..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn words(s: &str) -> usize {
        s.split_whitespace().count().max(1)
    }

    #[test]
    fn context_scales_with_spare_memory_and_respects_training() {
        assert_eq!(context_for_budget(4000, 12, 32768), MAX_CONTEXT);
        let tight = context_for_budget(200, 144, 32768);
        assert!(tight > MIN_CONTEXT && tight < MAX_CONTEXT);
        assert_eq!(context_for_budget(0, 144, 32768), MIN_CONTEXT);
        assert_eq!(context_for_budget(100_000, 12, 2048), 2048);
        assert!(
            context_for_budget(64, 12, 262_144) > context_for_budget(64, 144, 32_768),
            "a cheaper KV cache buys a bigger window"
        );
    }

    #[test]
    fn the_prompt_cap_always_leaves_room_to_answer() {
        for n in [MIN_CONTEXT, 1024, 2048, MAX_CONTEXT] {
            assert!(max_prompt_tokens(n) + reply_budget(n) as usize <= n as usize);
        }
        assert!(max_prompt_tokens(64) >= 16);
        assert_eq!(reply_budget(MIN_CONTEXT), 256);
        assert_eq!(reply_budget(MAX_CONTEXT), 1024);
    }

    #[test]
    fn history_is_kept_newest_first_whole_and_the_question_always() {
        let msgs = vec![
            Msg::system("sys"),
            Msg::user("oldest question here"),
            Msg::assistant("oldest answer here"),
            Msg::user("middle question here"),
            Msg::assistant("middle answer here"),
            Msg::user("what now"),
        ];
        let all = fit_messages(&msgs, 4096, 300, &words);
        assert_eq!(all.len(), 6);
        // Room for one turn: the newest, as a whole.
        let some = fit_messages(&msgs, 300 + 12 + 1 + 12 + 2 + 6 + 24 + 1, 300, &words);
        let texts: Vec<&str> = some.iter().map(|m| m.content.as_str()).collect();
        assert_eq!(
            texts,
            vec![
                "sys",
                "middle question here",
                "middle answer here",
                "what now"
            ]
        );
        // No room at all: system + the question only.
        let none = fit_messages(&msgs, 300, 300, &words);
        let texts: Vec<&str> = none.iter().map(|m| m.content.as_str()).collect();
        assert_eq!(texts, vec!["sys", "what now"]);
    }
}
