//! Framing a conversation for a model family. Getting this wrong yields
//! garbage, so the family travels with the model as data (`ModelRef::
//! prompt_style` in the manifest) rather than being assumed.

use verdure_core::{Msg, Role};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptStyle {
    /// Qwen 2.5 etc. — ChatML, no leading BOS.
    ChatMl,
    /// ChatML for a reasoning model (Qwen 3.5) with thinking turned OFF via a
    /// pre-filled empty `<think>` block — its documented "answer directly"
    /// switch. Without it the model spends the whole budget on scratchpad
    /// (measured on-device 2026-09-10, knowledge/03).
    ChatMlNoThink,
    /// Gemma — `<start_of_turn>` markers, leading BOS, no system role.
    Gemma,
}

impl PromptStyle {
    /// The manifest's spelling.
    pub fn parse(s: &str) -> Option<PromptStyle> {
        match s.trim().to_ascii_lowercase().as_str() {
            "chatml" => Some(PromptStyle::ChatMl),
            "chatml_nothink" | "chatml-nothink" => Some(PromptStyle::ChatMlNoThink),
            "gemma" => Some(PromptStyle::Gemma),
            _ => None,
        }
    }

    /// Whether the tokenizer should add a BOS token for this family.
    pub fn add_bos(self) -> bool {
        matches!(self, PromptStyle::Gemma)
    }
}

/// Render `messages` (system first if any, then alternating user/assistant,
/// ending with a user turn) as the model's prompt, cued for the assistant to
/// answer. Consecutive same-role messages are merged so the structure the
/// model relies on is never broken.
pub fn format_chat(messages: &[Msg], style: PromptStyle) -> String {
    let system: Vec<&str> = messages
        .iter()
        .filter(|m| m.role == Role::System)
        .map(|m| m.content.as_str())
        .collect();
    let system = system.join("\n\n");
    let turns: Vec<(Role, String)> =
        merge_turns(messages.iter().filter(|m| m.role != Role::System));

    let mut p = String::new();
    match style {
        PromptStyle::ChatMl | PromptStyle::ChatMlNoThink => {
            if !system.is_empty() {
                p.push_str(&format!("<|im_start|>system\n{system}<|im_end|>\n"));
            }
            for (role, content) in &turns {
                let r = if *role == Role::User {
                    "user"
                } else {
                    "assistant"
                };
                p.push_str(&format!("<|im_start|>{r}\n{content}<|im_end|>\n"));
            }
            p.push_str("<|im_start|>assistant\n");
            if style == PromptStyle::ChatMlNoThink {
                p.push_str("<think>\n\n</think>\n\n");
            }
        }
        PromptStyle::Gemma => {
            // No system role: the instruction rides the first user turn.
            let mut first_user = true;
            for (role, content) in &turns {
                match role {
                    Role::User => {
                        let body = if first_user && !system.is_empty() {
                            format!("{system}\n\n{content}")
                        } else {
                            content.clone()
                        };
                        first_user = false;
                        p.push_str(&format!("<start_of_turn>user\n{body}<end_of_turn>\n"));
                    }
                    _ => p.push_str(&format!("<start_of_turn>model\n{content}<end_of_turn>\n")),
                }
            }
            if first_user && !system.is_empty() {
                p.push_str(&format!("<start_of_turn>user\n{system}<end_of_turn>\n"));
            }
            p.push_str("<start_of_turn>model\n");
        }
    }
    p
}

fn merge_turns<'a>(msgs: impl Iterator<Item = &'a Msg>) -> Vec<(Role, String)> {
    let mut out: Vec<(Role, String)> = Vec::new();
    for m in msgs {
        match out.last_mut() {
            Some((role, content)) if *role == m.role => {
                content.push_str("\n\n");
                content.push_str(&m.content);
            }
            _ => out.push((m.role, m.content.clone())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convo() -> Vec<Msg> {
        vec![
            Msg::system("Be brief."),
            Msg::user("my name is Ada"),
            Msg::assistant("Nice to meet you, Ada."),
            Msg::user("what is my name?"),
        ]
    }

    #[test]
    fn chatml_frames_history_in_order_and_cues_the_assistant() {
        let p = format_chat(&convo(), PromptStyle::ChatMl);
        assert!(p.starts_with("<|im_start|>system\nBe brief.<|im_end|>\n"));
        assert!(p.find("my name is Ada").unwrap() < p.find("what is my name?").unwrap());
        assert!(p.ends_with("<|im_start|>assistant\n"));
        assert_eq!(
            p.matches("<|im_start|>").count(),
            p.matches("<|im_end|>").count() + 1,
            "balanced markers"
        );
        assert!(!PromptStyle::ChatMl.add_bos());
    }

    #[test]
    fn no_think_prefills_an_empty_thinking_block_and_otherwise_matches_chatml() {
        let p = format_chat(&convo(), PromptStyle::ChatMlNoThink);
        assert!(p.ends_with("<|im_start|>assistant\n<think>\n\n</think>\n\n"));
        assert_eq!(
            p.strip_suffix("<think>\n\n</think>\n\n").unwrap(),
            format_chat(&convo(), PromptStyle::ChatMl)
        );
    }

    #[test]
    fn gemma_puts_the_instruction_on_the_first_user_turn_once() {
        let p = format_chat(&convo(), PromptStyle::Gemma);
        assert!(p.starts_with("<start_of_turn>user\nBe brief.\n\nmy name is Ada<end_of_turn>\n"));
        assert_eq!(p.matches("Be brief.").count(), 1);
        assert!(p.contains("<start_of_turn>model\nNice to meet you, Ada.<end_of_turn>\n"));
        assert!(p.ends_with("<start_of_turn>model\n"));
        assert!(!p.contains("<|im_start|>") && !p.contains("<bos>"));
        assert!(PromptStyle::Gemma.add_bos());
    }

    #[test]
    fn consecutive_same_role_messages_merge_and_styles_parse() {
        let msgs = vec![Msg::user("Schema: …"), Msg::user("Text: …")];
        let p = format_chat(&msgs, PromptStyle::ChatMl);
        assert_eq!(p.matches("<|im_start|>user").count(), 1);
        assert!(p.contains("Schema: …\n\nText: …"));
        assert_eq!(PromptStyle::parse("ChatML"), Some(PromptStyle::ChatMl));
        assert_eq!(
            PromptStyle::parse("chatml_nothink"),
            Some(PromptStyle::ChatMlNoThink)
        );
        assert_eq!(PromptStyle::parse("gemma"), Some(PromptStyle::Gemma));
        assert_eq!(PromptStyle::parse("llama3"), None);
    }
}
