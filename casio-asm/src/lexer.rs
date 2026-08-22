#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    // keywords
    Model,
    Opn,
    Csc,
    If,
    Else,
    While,
    For,
    Asm,
    Let,
    Var,
    Return,
    Goto,
    // types
    U8,
    U16,
    U32,
    U64,
    Void,
    // literals
    Ident(String),
    Number(u32),      // decimal
    Addr(u32),        // [ABCD] -> value
    Str(String),
    // symbols
    At,               // @
    LBracket,         // [
    RBracket,         // ]
    LParen,           // (
    RParen,           // )
    LBrace,           // {
    RBrace,           // }
    Semi,             // ;
    Comma,            // ,
    Colon,            // :
    Eq,               // =
    EqEq,             // ==
    NotEq,            // !=
    Lt,               // <
    Gt,               // >
    LtEq,             // <=
    GtEq,             // >=
    Plus,             // +
    Minus,            // -
    Star,             // *
    Slash,            // /
    Percent,          // %
    Amp,              // &
    Pipe,             // |
    Caret,            // ^
    Bang,             // !
    LtLt,             // <<
    GtGt,             // >>
    AmpAmp,           // &&
    PipePipe,         // ||
    PlusPlus,         // ++
    MinusMinus,       // --
    PlusEq,           // +=
    MinusEq,          // -=
    Eof,
}

pub fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() || c == '\u{feff}' {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        if c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    let n = chars[i + 1];
                    match n {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        _ => s.push(n),
                    }
                    i += 2;
                } else {
                    s.push(chars[i]);
                    i += 1;
                }
            }
            i += 1; // closing "
            out.push(Tok::Str(s));
            continue;
        }
        if c == '[' {
            // try to parse [HEX] as Addr token
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let start = j;
            while j < chars.len() && chars[j].is_ascii_hexdigit() {
                j += 1;
            }
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && chars[j] == ']' && j > start {
                let hex: String = chars[start..j].iter().filter(|ch| ch.is_ascii_hexdigit()).collect();
                if let Ok(v) = u32::from_str_radix(&hex, 16) {
                    out.push(Tok::Addr(v));
                    i = j + 1;
                    continue;
                }
            }
            out.push(Tok::LBracket);
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            let s: String = chars[i..j].iter().collect();
            let v = s.parse::<u32>().map_err(|e| format!("number {s}: {e}"))?;
            out.push(Tok::Number(v));
            i = j;
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let w: String = chars[i..j].iter().collect();
            let tok = match w.as_str() {
                "model" => Tok::Model,
                "opn" => Tok::Opn,
                "csc" => Tok::Csc,
                "if" => Tok::If,
                "else" => Tok::Else,
                "while" => Tok::While,
                "for" => Tok::For,
                "asm" => Tok::Asm,
                "let" => Tok::Let,
                "var" => Tok::Var,
                "return" => Tok::Return,
                "goto" => Tok::Goto,
                "u8" => Tok::U8,
                "u16" => Tok::U16,
                "u32" => Tok::U32,
                "u64" => Tok::U64,
                "int" => Tok::U16,
                "at" => Tok::At,
                "void" => Tok::Void,
                _ => Tok::Ident(w),
            };
            out.push(tok);
            i = j;
            continue;
        }
        // multi-char symbols
        let two = if i + 1 < chars.len() {
            format!("{}{}", c, chars[i + 1])
        } else {
            String::new()
        };
        match two.as_str() {
            "==" => { out.push(Tok::EqEq); i += 2; continue; }
            "!=" => { out.push(Tok::NotEq); i += 2; continue; }
            "<=" => { out.push(Tok::LtEq); i += 2; continue; }
            ">=" => { out.push(Tok::GtEq); i += 2; continue; }
            "<<" => { out.push(Tok::LtLt); i += 2; continue; }
            ">>" => { out.push(Tok::GtGt); i += 2; continue; }
            "&&" => { out.push(Tok::AmpAmp); i += 2; continue; }
            "||" => { out.push(Tok::PipePipe); i += 2; continue; }
            "++" => { out.push(Tok::PlusPlus); i += 2; continue; }
            "--" => { out.push(Tok::MinusMinus); i += 2; continue; }
            "+=" => { out.push(Tok::PlusEq); i += 2; continue; }
            "-=" => { out.push(Tok::MinusEq); i += 2; continue; }
            _ => {}
        }
        let tok = match c {
            '@' => Tok::At,
            ']' => Tok::RBracket,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '{' => Tok::LBrace,
            '}' => Tok::RBrace,
            ';' => Tok::Semi,
            ',' => Tok::Comma,
            ':' => Tok::Colon,
            '=' => Tok::Eq,
            '<' => Tok::Lt,
            '>' => Tok::Gt,
            '+' => Tok::Plus,
            '-' => Tok::Minus,
            '*' => Tok::Star,
            '/' => Tok::Slash,
            '%' => Tok::Percent,
            '&' => Tok::Amp,
            '|' => Tok::Pipe,
            '^' => Tok::Caret,
            '!' => Tok::Bang,
            _ => return Err(format!("unexpected char {c:?} at {i}")),
        };
        out.push(tok);
        i += 1;
    }
    out.push(Tok::Eof);
    Ok(out)
}
