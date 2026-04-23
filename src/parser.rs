use std::iter::Peekable;
use std::str::Chars;

/// Remove a camada externa de string escapada (se existir).
/// Caso o input seja um JSON string (ex: "\"{ \\\"code\\\": [...] }\""),
/// desserializa para obter o conteúdo bruto.
fn unescape_outer(input: &str) -> String {
    match serde_json::from_str::<String>(input) {
        Ok(v) => v,
        Err(_) => input.to_string(),
    }
}

/// Retorna o próximo caractere não-whitespace sem consumir o iterador.
fn next_significant(chars: &mut Peekable<Chars>) -> Option<char> {
    let mut clone = chars.clone();
    while let Some(c) = clone.next() {
        if !c.is_whitespace() {
            return Some(c);
        }
    }
    None
}

/// Parser manual robusto: localiza a chave `"code"` no JSON e extrai
/// todos os itens do array de strings associado, respeitando escapes
/// (\\n, \\t, \\r, \\", \\\\) e sem truncar o conteúdo.
fn extract_code_array(input: &str) -> Vec<String> {
    let mut result = Vec::new();

    // 1. Localiza a chave "code"
    let start = match input.find("\"code\"") {
        Some(pos) => pos,
        None => {
            eprintln!("[parser] key \"code\" not found in input");
            return result;
        }
    };

    // 2. Localiza o início do array `[`
    let after_key = &input[start..];
    let array_start = match after_key.find('[') {
        Some(pos) => pos,
        None => {
            eprintln!("[parser] array '[' not found after \"code\" key");
            return result;
        }
    };

    let mut chars = after_key[array_start + 1..].chars().peekable();
    let mut current = String::new();
    let mut inside_string = false;
    let mut escape = false;
    let mut array_depth = 1;

    while let Some(c) = chars.next() {
        if inside_string {
            if escape {
                match c {
                    'n'  => current.push('\n'),
                    't'  => current.push('\t'),
                    'r'  => current.push('\r'),
                    '"'  => current.push('"'),
                    '\\' => current.push('\\'),
                    // Qualquer outro escape: preserva literalmente
                    _    => {
                        current.push('\\');
                        current.push(c);
                    }
                }
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                // Fechamento inteligente: só fecha a string se o próximo
                // token significativo for ',' ou ']' (fim de elemento).
                match next_significant(&mut chars) {
                    Some(',') | Some(']') => {
                        result.push(current.clone());
                        current.clear();
                        inside_string = false;
                    }
                    _ => {
                        // Aspas internas ao conteúdo — preserva
                        current.push('"');
                    }
                }
            } else {
                current.push(c);
            }
        } else {
            match c {
                '"'  => { inside_string = true; }
                '['  => { array_depth += 1; }
                ']'  => {
                    array_depth -= 1;
                    if array_depth == 0 {
                        break;
                    }
                }
                _    => {}
            }
        }
    }

    result
}

/// Ponto de entrada público: recebe o conteúdo bruto vindo do LLM e
/// retorna as classes Java extraídas do array `"code"`.
pub fn extract_codes(raw: &str) -> Result<Vec<String>, ParserError> {
    let trimmed = raw.trim();
    let unescaped = unescape_outer(trimmed);

    let codes = extract_code_array(&unescaped);

    if codes.is_empty() {
        eprintln!("[parser] ❌ no codes found in LLM output");
        return Err(ParserError::CodeNotFound);
    }

    eprintln!("[parser] ✅ {} code(s) extracted", codes.len());
    Ok(codes)
}

// ------------------------------------------------------------------
// ERROR HANDLING
// ------------------------------------------------------------------

#[derive(Debug)]
pub enum ParserError {
    CodeNotFound,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::CodeNotFound => write!(f, "No codes found in LLM output"),
        }
    }
}

impl std::error::Error for ParserError {}