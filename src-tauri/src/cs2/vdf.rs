//! Minimal Valve KeyValues (text VDF) parser.
//!
//! Supports quoted and unquoted tokens, escape sequences inside quotes,
//! nested blocks, `//` comments and `[$CONDITIONAL]` suffixes (ignored).

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

impl Vdf {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Obj(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(v) => v,
            Vdf::Str(_) => &[],
        }
    }

    /// Case-insensitive child lookup (first match).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        self.entries()
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    /// Case-insensitive lookup through a path of keys.
    pub fn path(&self, keys: &[&str]) -> Option<&Vdf> {
        keys.iter().try_fold(self, |node, k| node.get(k))
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Vdf::as_str)
    }

    /// Flatten string children of this object into a map (last wins).
    #[allow(dead_code)]
    pub fn string_map(&self) -> BTreeMap<String, String> {
        self.entries()
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect()
    }
}

#[derive(Debug, PartialEq)]
enum Tok {
    Str(String),
    Open,
    Close,
}

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let mut it = src.chars().peekable();
    while let Some(&c) = it.peek() {
        match c {
            c if c.is_whitespace() => {
                it.next();
            }
            '/' => {
                it.next();
                if it.peek() == Some(&'/') {
                    for c in it.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                } else {
                    // A lone slash is part of an unquoted token.
                    let mut s = String::from('/');
                    while let Some(&c) = it.peek() {
                        if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                            break;
                        }
                        s.push(c);
                        it.next();
                    }
                    out.push(Tok::Str(s));
                }
            }
            '{' => {
                it.next();
                out.push(Tok::Open);
            }
            '}' => {
                it.next();
                out.push(Tok::Close);
            }
            '"' => {
                it.next();
                let mut s = String::new();
                let mut closed = false;
                while let Some(c) = it.next() {
                    match c {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => match it.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('\\') => s.push('\\'),
                            Some('"') => s.push('"'),
                            Some(o) => {
                                s.push('\\');
                                s.push(o);
                            }
                            None => break,
                        },
                        o => s.push(o),
                    }
                }
                if !closed {
                    return Err("string sem aspas de fecho".into());
                }
                out.push(Tok::Str(s));
            }
            '[' => {
                // Conditional like [$WIN32] — skip it.
                for c in it.by_ref() {
                    if c == ']' {
                        break;
                    }
                }
            }
            _ => {
                let mut s = String::new();
                while let Some(&c) = it.peek() {
                    if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                        break;
                    }
                    s.push(c);
                    it.next();
                }
                out.push(Tok::Str(s));
            }
        }
    }
    Ok(out)
}

/// Parse a VDF document. The result is an object holding the top-level pairs.
pub fn parse(src: &str) -> Result<Vdf, String> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let toks = tokenize(src)?;
    let mut pos = 0;
    let root = parse_obj(&toks, &mut pos, true)?;
    Ok(Vdf::Obj(root))
}

fn parse_obj(toks: &[Tok], pos: &mut usize, top: bool) -> Result<Vec<(String, Vdf)>, String> {
    let mut items = Vec::new();
    loop {
        match toks.get(*pos) {
            None if top => return Ok(items),
            None => return Err("bloco sem '}' de fecho".into()),
            Some(Tok::Close) if !top => {
                *pos += 1;
                return Ok(items);
            }
            Some(Tok::Close) => return Err("'}' inesperado".into()),
            Some(Tok::Open) => return Err("'{' inesperado".into()),
            Some(Tok::Str(k)) => {
                *pos += 1;
                match toks.get(*pos) {
                    Some(Tok::Str(v)) => {
                        *pos += 1;
                        items.push((k.clone(), Vdf::Str(v.clone())));
                    }
                    Some(Tok::Open) => {
                        *pos += 1;
                        let child = parse_obj(toks, pos, false)?;
                        items.push((k.clone(), Vdf::Obj(child)));
                    }
                    _ => return Err(format!("valor em falta para a chave '{k}'")),
                }
            }
        }
    }
}

/// Quote a string for writing VDF / cfg text.
pub fn quote(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_and_comments() {
        let src = r#"
        // header comment
        "libraryfolders"
        {
            "0"
            {
                "path"		"C:\\Program Files (x86)\\Steam"
                "apps"
                {
                    "730"		"123"
                }
            }
            "1" { "path" "D:\\Games" } // trailing
        }"#;
        let v = parse(src).unwrap();
        let lf = v.get("LibraryFolders").unwrap();
        assert_eq!(lf.path(&["0", "path"]).unwrap().as_str(), Some(r"C:\Program Files (x86)\Steam"));
        assert_eq!(lf.path(&["0", "apps", "730"]).unwrap().as_str(), Some("123"));
        assert_eq!(lf.path(&["1", "path"]).unwrap().as_str(), Some(r"D:\Games"));
    }

    #[test]
    fn escaped_quotes_and_unquoted() {
        let v = parse(r#"root { key "a \"quoted\" value" bare token [$WIN32] }"#).unwrap();
        let r = v.get("root").unwrap();
        assert_eq!(r.get_str("key"), Some(r#"a "quoted" value"#));
        assert_eq!(r.get_str("bare"), Some("token"));
    }

    #[test]
    fn errors_on_unclosed() {
        assert!(parse(r#""a" { "b" "c" "#).is_err());
        assert!(parse(r#""a" "unterminated"#).is_err());
    }

    #[test]
    fn quote_roundtrip() {
        let q = quote(r#"he said "hi" \ ok"#);
        let v = parse(&format!("k {q}")).unwrap();
        assert_eq!(v.get_str("k"), Some(r#"he said "hi" \ ok"#));
    }

    #[test]
    fn bom_is_ignored() {
        let v = parse("\u{feff}\"a\" \"b\"").unwrap();
        assert_eq!(v.get_str("a"), Some("b"));
    }
}
