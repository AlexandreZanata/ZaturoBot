//! Fast path for explicit owner commands. Unknown/conditional prose never authorizes tools.
#[derive(Debug, PartialEq, Eq)]
pub enum ChatIntent {
    Computer { request: String },
    PriceSearch { query: String },
    ComputerApproval { id: String, allow: bool },
    Diagnostic,
    YoutubeProfiles,
    AndroidDevices,
    YoutubeGenerate { channel: String, count: u32 },
    Shared { instructions: String },
    AndroidOpenMessages,
    Conversation,
}
pub fn chat_intent(message: &str) -> ChatIntent {
    let trimmed = message.trim();
    for (prefix, allow) in [("/aprovar ", true), ("/recusar ", false)] {
        if let Some(id) = trimmed.strip_prefix(prefix)
            && crate::service::canonical_operation_id(id)
        {
            return ChatIntent::ComputerApproval {
                id: id.into(),
                allow,
            };
        }
    }
    for prefix in ["/computador ", "no computador: ", "tarefa no pc: "] {
        if let Some(request) = trimmed.strip_prefix(prefix)
            && crate::ordinary_text(request, 2048)
        {
            return ChatIntent::Computer {
                request: request.into(),
            };
        }
    }
    for prefix in [
        "/precos ",
        "/preços ",
        "pesquise no bitlupa ",
        "pesquisar no bitlupa ",
    ] {
        if let Some(query) = trimmed.strip_prefix(prefix)
            && crate::ordinary_text(query, 256)
        {
            return ChatIntent::PriceSearch {
                query: query.into(),
            };
        }
    }
    // The owner's example remains an exact read-only product request, never an upgrade to Pro.
    // Preserve byte offsets into the original product query across Unicode case expansion.
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("pesquise ")
        && lower.contains("bitlupa")
        && let Some(end) = lower.find(" usando ")
    {
        let mut query = &trimmed["pesquise ".len()..end];
        for prefix in ["o celular ", "o smartphone ", "celular ", "smartphone "] {
            if query.to_ascii_lowercase().starts_with(prefix) {
                query = &query[prefix.len()..];
                break;
            }
        }
        if crate::ordinary_text(query, 256) {
            return ChatIntent::PriceSearch {
                query: query.into(),
            };
        }
    }

    for prefix in [
        "/compartilhar ",
        "compartilhe com o codex: ",
        "deixe para o codex: ",
    ] {
        if message.trim_start().to_lowercase().starts_with(prefix) {
            let instructions = message.trim_start()[prefix.len()..].trim();
            if crate::ordinary_text(instructions, 4096) {
                return ChatIntent::Shared {
                    instructions: instructions.into(),
                };
            }
        }
    }
    let normalized = normalize_owner_text(message);
    let text = normalized.trim().trim_end_matches(['.', '!']);
    explicit_normalized(text)
}

pub(crate) fn normalize_owner_text(message: &str) -> String {
    message
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' => 'u',
            'ç' => 'c',
            _ => c,
        })
        .collect::<String>()
}

fn explicit_normalized(text: &str) -> ChatIntent {
    match text {
        "diagnostico" | "/diagnostico" | "status do pc" => return ChatIntent::Diagnostic,
        "canais" | "listar canais" | "/canais" => return ChatIntent::YoutubeProfiles,
        "celulares" | "listar celulares" | "/celulares" => return ChatIntent::AndroidDevices,
        "abrir sms" | "abra as mensagens do celular" => return ChatIntent::AndroidOpenMessages,
        _ => {}
    }
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.len() < 4
        || !matches!(
            words[0],
            "faca" | "fazer" | "crie" | "criar" | "gere" | "gerar" | "produza"
        )
        || !matches!(words[2], "video" | "videos")
    {
        return ChatIntent::Conversation;
    }
    let count = match words[1] {
        "um" => 1,
        "dois" => 2,
        "tres" => 3,
        "quatro" => 4,
        "cinco" => 5,
        "seis" => 6,
        "sete" => 7,
        "oito" => 8,
        "nove" => 9,
        "dez" => 10,
        v => v.parse().unwrap_or(0),
    };
    if !(1..=10).contains(&count) {
        return ChatIntent::Conversation;
    }
    let mut channel = words[3..].to_vec();
    while channel
        .first()
        .is_some_and(|v| matches!(*v, "do" | "da" | "para" | "o" | "canal"))
    {
        channel.remove(0);
    }
    let Some(slug) = canonical_youtube_channel(&channel.join(" ")) else {
        return ChatIntent::Conversation;
    };
    ChatIntent::YoutubeGenerate {
        channel: slug.into(),
        count,
    }
}

pub fn canonical_youtube_channel(alias: &str) -> Option<&'static str> {
    match alias {
        "fallem prince" | "fallem price" | "falllem price" | "fallem-prince" | "fallemprince" => {
            Some("fallem-prince")
        }
        "whispers from heaven" | "whispers-from-heaven" => Some("whispers-from-heaven"),
        "zaflas" => Some("zaflas"),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn price_example_preserves_unicode_query_byte_boundaries() {
        assert_eq!(
            chat_intent("pesquise İphone usando BitLupa"),
            ChatIntent::PriceSearch {
                query: "İphone".into()
            }
        );
        assert_eq!(
            chat_intent("pesquise o celular POCO X7 usando o projeto BitLupa"),
            ChatIntent::PriceSearch {
                query: "POCO X7".into()
            }
        );
    }
    #[test]
    fn explicit_commands_have_a_zero_model_fast_path() {
        assert_eq!(
            chat_intent("faça 2 videos para o falllem price"),
            ChatIntent::YoutubeGenerate {
                channel: "fallem-prince".into(),
                count: 2
            }
        );
        assert_eq!(
            chat_intent("faça 4 videos do fallem price"),
            ChatIntent::YoutubeGenerate {
                channel: "fallem-prince".into(),
                count: 4
            }
        );
        assert_eq!(
            chat_intent("Gere quatro vídeos para o canal fallem prince!"),
            ChatIntent::YoutubeGenerate {
                channel: "fallem-prince".into(),
                count: 4
            }
        );
        for text in [
            "não faça 4 videos do fallem price",
            "se eu pedir faça 4 videos do fallem price",
            "faça 4 videos do fallem price e publique",
            "faça 999 videos do fallem price",
            "faça 4 videos do canal desconhecido",
            "faça -2 videos para o falllem price",
            "faça 2.5 videos para o falllem price",
            "faça dois videos para o falllem price e agende",
            "se pudesse, faça 2 videos para o falllem price",
            "não faça 2 videos para o falllem price",
            "faça 2 videos para o falllem price e zaflas",
        ] {
            assert_eq!(chat_intent(text), ChatIntent::Conversation);
        }
    }
}

/// A private-view prompt, never consent to navigate, infer, watch SMS or send a message.
pub fn requested_phone_capture(message: &str) -> bool {
    if !crate::ordinary_text(message, 4096) {
        return false;
    }
    let normalized = normalize_owner_text(message);
    let words = normalized
        .split(|c: char| !c.is_alphanumeric())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    let capture = words.iter().position(|v| {
        matches!(
            *v,
            "print" | "prints" | "captura" | "capture" | "capturar" | "screenshot"
        )
    });
    let Some(capture) = capture else {
        return false;
    };
    let prefix = &words[..=capture];
    let negated = prefix.iter().enumerate().any(|(i, v)| {
        matches!(*v, "nao" | "nunca" | "sem")
            && prefix[i + 1..capture].iter().all(|word| {
                matches!(
                    *word,
                    "me" | "o"
                        | "a"
                        | "um"
                        | "uma"
                        | "envie"
                        | "enviar"
                        | "mande"
                        | "mandar"
                        | "tire"
                        | "tirar"
                        | "faca"
                        | "fazer"
                        | "quero"
                        | "que"
                        | "voce"
                        | "preciso"
                )
            })
    });
    if matches!(words.first(), Some(&"se" | &"talvez" | &"poderia")) || negated {
        return false;
    }
    words.iter().any(|v| {
        matches!(
            *v,
            "sms" | "sm" | "mensagem" | "mensagens" | "celular" | "telefone" | "smartphone"
        )
    })
}

#[cfg(test)]
mod phone_capture_tests {
    use super::*;
    #[test]
    fn screenshot_requests_are_not_navigation_or_sms_events() {
        for text in [
            "envie print do sms",
            "enviar print do sm",
            "mande um print da mensagem",
            "print da tela atual do celular",
            "envie print do sms mesmo que nenhum SMS foi enviado",
            "mande print do SMS que não chegou",
            "não envie SMS, mande print do celular",
            "o SMS não chegou, envie print do celular",
        ] {
            assert!(requested_phone_capture(text), "{text}");
            assert!(!matches!(
                chat_intent(text),
                ChatIntent::AndroidOpenMessages
            ));
        }
        for text in [
            "abrir sms",
            "não envie print do SMS",
            "nunca tire um print do celular",
            "se chegar SMS envie print",
            "envie print do computador",
        ] {
            assert!(!requested_phone_capture(text), "{text}");
        }
    }
}
