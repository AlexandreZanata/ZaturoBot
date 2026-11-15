//! Bounded model proposals over registered actions. Proposals never grant additional operations.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IntentDecision {
    Reply { text: String },
    Clarify { question: String },
    Action { action: RegisteredAction },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum RegisteredAction {
    YoutubeGenerate { channel: String, count: u32 },
    BrowserResearch { request: String },
    AndroidDevices,
    AndroidOpenMessages { device_id: String },
}

impl IntentDecision {
    pub fn validates_for(&self, owner_message: &str) -> bool {
        if !crate::ordinary_text(owner_message, 2048) {
            return false;
        }
        match self {
            Self::Reply { text } => crate::ordinary_text(text, 1500),
            Self::Clarify { question } => crate::ordinary_text(question, 500),
            Self::Action { action } => action.authorized_by(owner_message),
        }
    }
}

impl RegisteredAction {
    pub fn authorized_by(&self, message: &str) -> bool {
        let text = crate::intent::normalize_owner_text(message);
        let words = text
            .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '?' | '!'))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        let denied = words.iter().any(|word| {
            matches!(
                *word,
                "nao"
                    | "not"
                    | "don't"
                    | "never"
                    | "se"
                    | "if"
                    | "hipoteticamente"
                    | "hypothetically"
                    | "exemplo"
                    | "example"
                    | "simule"
                    | "simulate"
            )
        });
        if denied {
            return false;
        }
        match self {
            Self::YoutubeGenerate { channel, count } => {
                if !(1..=10).contains(count)
                    || crate::intent::canonical_youtube_channel(channel) != Some(channel.as_str())
                {
                    return false;
                }
                if words.iter().any(|word| {
                    matches!(*word, "e" | "and" | "tambem" | "also")
                        || [
                            "public", "agend", "upload", "schedul", "delete", "apague", "envie",
                            "send",
                        ]
                        .iter()
                        .any(|prefix| word.starts_with(prefix))
                }) {
                    return false;
                }
                let counts = words
                    .iter()
                    .filter_map(|word| count_word(word))
                    .collect::<Vec<_>>();
                if counts != [*count] {
                    return false;
                }
                if !words.iter().any(|word| {
                    matches!(
                        *word,
                        "faca"
                            | "fazer"
                            | "crie"
                            | "criar"
                            | "gere"
                            | "gerar"
                            | "produza"
                            | "produzir"
                            | "make"
                            | "create"
                            | "generate"
                    )
                }) || !words
                    .iter()
                    .any(|word| matches!(*word, "video" | "videos" | "videos."))
                {
                    return false;
                }
                let padded = format!(" {} ", text.trim().trim_end_matches(['.', '!', '?']));
                let mut remainder = padded.clone();
                let mut channels = Vec::new();
                for alias in [
                    "fallem prince",
                    "fallem price",
                    "falllem price",
                    "fallem-prince",
                    "fallemprince",
                    "whispers from heaven",
                    "whispers-from-heaven",
                    "zaflas",
                ] {
                    if padded.contains(&format!(" {alias} ")) {
                        let canonical = crate::intent::canonical_youtube_channel(alias).unwrap();
                        if !channels.contains(&canonical) {
                            channels.push(canonical);
                        }
                        remainder = remainder.replace(&format!(" {alias} "), " ");
                    }
                }
                channels == [channel.as_str()]
                    && remainder.split_whitespace().all(|word| {
                        let word = word.trim_matches([',', '.', ';', '?', '!']);
                        count_word(word).is_some()
                            || matches!(
                                word,
                                "por"
                                    | "favor"
                                    | "voce"
                                    | "pode"
                                    | "poderia"
                                    | "quero"
                                    | "preciso"
                                    | "que"
                                    | "de"
                                    | "me"
                                    | "para"
                                    | "o"
                                    | "a"
                                    | "do"
                                    | "da"
                                    | "canal"
                                    | "faca"
                                    | "fazer"
                                    | "crie"
                                    | "criar"
                                    | "gere"
                                    | "gerar"
                                    | "produza"
                                    | "produzir"
                                    | "make"
                                    | "create"
                                    | "generate"
                                    | "please"
                                    | "can"
                                    | "you"
                                    | "video"
                                    | "videos"
                                    | "hoje"
                                    | "agora"
                                    | "today"
                                    | "now"
                            )
                    })
            }
            Self::BrowserResearch { request } => {
                request == message.trim()
                    && crate::ordinary_text(request, 2048)
                    && !words.iter().any(|word| {
                        matches!(
                            *word,
                            "compre"
                                | "publique"
                                | "poste"
                                | "agende"
                                | "envie"
                                | "apague"
                                | "desbloqueie"
                                | "buy"
                                | "purchase"
                                | "publish"
                                | "schedule"
                                | "send"
                                | "delete"
                                | "unlock"
                        )
                    })
                    && ["pesquis", "busqu", "preco", "research", "search"]
                        .iter()
                        .any(|term| text.contains(term))
            }
            Self::AndroidDevices => {
                matches!(
                    crate::chat_intent(message),
                    crate::ChatIntent::AndroidDevices
                ) || (text.contains("celular")
                    && ["conectad", "usb", "dispositiv"]
                        .iter()
                        .any(|term| text.contains(term)))
            }
            Self::AndroidOpenMessages { device_id } => {
                device_id == "owner-phone"
                    && matches!(
                        crate::chat_intent(message),
                        crate::ChatIntent::AndroidOpenMessages
                    )
            }
        }
    }
}

fn count_word(word: &str) -> Option<u32> {
    match word {
        "um" | "one" => Some(1),
        "dois" | "two" => Some(2),
        "tres" | "three" => Some(3),
        "quatro" | "four" => Some(4),
        "cinco" | "five" => Some(5),
        "seis" | "six" => Some(6),
        "sete" | "seven" => Some(7),
        "oito" | "eight" => Some(8),
        "nove" | "nine" => Some(9),
        "dez" | "ten" => Some(10),
        _ => word.parse().ok(),
    }
}

/// Retrieve only relevant registered schemas, never arbitrary command strings or private profiles.
pub fn relevant_action_catalog(message: &str) -> serde_json::Value {
    let text = crate::intent::normalize_owner_text(message);
    let mut schemas = Vec::new();
    if text.contains("video")
        || text.contains("fallem")
        || text.contains("falllem")
        || text.contains("zaflas")
        || text.contains("whispers")
    {
        schemas.push(serde_json::json!({"action":"youtube_generate","capability":crate::ServiceRequest::YoutubeOpenCodeGenerate{channel:"fallem-prince".into(),count:1}.capability(),"arguments":{"channel":["fallem-prince","whispers-from-heaven","zaflas"],"count":{"minimum":1,"maximum":10}},"scope":"generation only through OpenCode; never publication or scheduling"}));
    }
    if ["pesquis", "busqu", "preco", "research", "search"]
        .iter()
        .any(|term| text.contains(term))
    {
        schemas.push(serde_json::json!({"action":"browser_research","capability":"computer.agent.v1","arguments":{"request":"exact original owner message"},"scope":"public browser research; executor independently enforces origins and native input approval"}));
    }
    if text.contains("celular")
        || text.contains("android")
        || text.contains("usb")
        || text.contains("sms")
    {
        schemas.push(serde_json::json!({"action":"android_devices","capability":"android.devices.v1","arguments":{}}));
        schemas.push(serde_json::json!({"action":"android_open_messages","capability":"android.messages.v1","arguments":{"device_id":["owner-phone"]},"scope":"explicit app navigation only; no message sending or screenshot authorization"}));
    }
    serde_json::json!(schemas)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn generation() -> RegisteredAction {
        RegisteredAction::YoutubeGenerate {
            channel: "fallem-prince".into(),
            count: 2,
        }
    }
    #[test]
    fn a_model_cannot_promote_negations_hypotheticals_bad_counts_or_extra_operations() {
        for message in [
            "não faça 2 videos para o falllem price",
            "se eu pedir faça 2 videos para o falllem price",
            "faça -2 videos para o falllem price",
            "faça 2.5 videos para o falllem price",
            "faça 999 videos para o falllem price",
            "faça 2 videos para o falllem price e publique",
            "faça 2 videos para o falllem price e agende",
            "faça 2 videos para o falllem price e zaflas",
            "faça 2 videos para canal desconhecido",
            "faça 2 videos para o fallem prince pro",
            "faça 2 videos para o falllem price e abra mensagens",
        ] {
            assert!(!generation().authorized_by(message), "{message}");
        }
        assert!(generation().authorized_by("faça 2 videos para o falllem price"));
        assert!(
            generation()
                .authorized_by("por favor, você pode gerar dois videos para o fallem prince?")
        );
    }
    #[test]
    fn malformed_unregistered_actions_and_unrelated_catalogs_are_rejected() {
        assert!(
            serde_json::from_str::<IntentDecision>(
                r#"{"kind":"action","action":{"action":"shell","command":"arbitrary"}}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<IntentDecision>(r#"{"kind":"action","action":{"action":"youtube_generate","channel":"fallem-prince","count":2,"approved_schedule":true}}"#).is_err());
        assert_eq!(relevant_action_catalog("Olá"), serde_json::json!([]));
        let catalog = relevant_action_catalog("faça 2 videos para o falllem price");
        assert_eq!(catalog.as_array().unwrap().len(), 1);
        assert!(!catalog.to_string().contains("android"));
        assert!(
            !RegisteredAction::AndroidOpenMessages {
                device_id: "other-phone".into()
            }
            .authorized_by("abrir sms")
        );
    }
}
