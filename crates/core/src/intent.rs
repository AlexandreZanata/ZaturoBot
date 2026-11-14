//! Fast path for explicit owner commands. Unknown/conditional prose never authorizes tools.
#[derive(Debug, PartialEq, Eq)]
pub enum ChatIntent {
    Diagnostic,
    YoutubeProfiles,
    AndroidDevices,
    YoutubeGenerate { channel: String, count: u32 },
    Shared { instructions: String },
    AndroidOpenMessages,
    Conversation,
}
pub fn chat_intent(message: &str) -> ChatIntent {
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
    let normalized = message
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
        .collect::<String>();
    let text = normalized.trim().trim_end_matches(['.', '!']);
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
    let slug = match channel.join(" ").as_str() {
        "fallem prince" | "fallem price" | "fallem-prince" | "fallemprince" => "fallem-prince",
        "whispers from heaven" | "whispers-from-heaven" => "whispers-from-heaven",
        "zaflas" => "zaflas",
        _ => return ChatIntent::Conversation,
    };
    ChatIntent::YoutubeGenerate {
        channel: slug.into(),
        count,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_commands_have_a_zero_model_fast_path() {
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
        ] {
            assert_eq!(chat_intent(text), ChatIntent::Conversation);
        }
    }
}
