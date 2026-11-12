// SPDX-License-Identifier: GPL-3.0-or-later
//! Xschem `{...}` braced-string and `key=value` property tokenizer/writer.
//!
//! Clean-room Rust port of the string and attribute lexing rules from xschem's
//! `src/save.c` (`load_ascii_string`, `save_ascii_string`) and `src/token.c`
//! (`get_tok_value`, `list_tokens`) by Stefan Frederik Schippers (GPL-2.0-or-later).

use ferroschem_core::Properties;

use crate::{FormatError, ParseLimits};

/// Read a `{...}` braced string from the front of `input`, returning the
/// unescaped string content and the remaining unparsed slice of `input`.
///
/// Follows the exact rules of xschem's `load_ascii_string` (`src/save.c`):
/// - Skips any leading whitespace (`' '`, `'\t'`, `'\r'`, `'\n'`) before `'{'`.
/// - Strips `'\r'` (carriage returns) inside the string so CRLF files normalize to LF.
/// - Inside `{...}`, `\` escapes the next character (`\\` -> `\`, `\{` -> `{`, `\}` -> `}`).
/// - Returns [`FormatError::Syntax`] if `'{'` is missing or EOF is reached before
///   an unescaped `'}'`.
pub fn read_braced_string(input: &str) -> Result<(String, &str), FormatError> {
    let max = ParseLimits::DEFAULT.max_string_bytes;
    if input.len() > max {
        return Err(FormatError::InputTooLarge {
            size: input.len(),
            max,
        });
    }

    let trimmed = input.trim_start_matches([' ', '\t', '\r', '\n']);
    let Some(after_open) = trimmed.strip_prefix('{') else {
        return Err(FormatError::Syntax(
            "expected '{' at start of braced string".into(),
        ));
    };

    let mut out = String::new();
    let mut chars = after_open.char_indices();
    let mut escape = false;

    for (idx, ch) in &mut chars {
        if ch == '\r' {
            continue;
        }
        if !escape {
            if ch == '}' {
                let rest = &after_open[idx + ch.len_utf8()..];
                return Ok((out, rest));
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
        }
        out.push(ch);
        escape = false;
    }

    Err(FormatError::Syntax(
        "unterminated '{...}' string (missing closing '}')".into(),
    ))
}

/// Format `text` as an xschem `{...}` braced string, escaping `\`, `{`, and `}`
/// with a leading backslash.
///
/// Follows the exact rules of xschem's `save_ascii_string` (`src/save.c`).
#[must_use]
pub fn write_braced_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('{');
    for ch in text.chars() {
        if matches!(ch, '\\' | '{' | '}') {
            out.push('\\');
        }
        out.push(ch);
    }
    out.push('}');
    out
}

/// Returns `true` if `ch` is a token separator in xschem property blocks
/// (`SPACE(c)` in `reference/xschem/src/token.c`).
#[inline]
fn is_prop_space(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | ' ' | '\t' | ';')
}

/// Parse the inner text of an xschem `{...}` property block into insertion-ordered
/// [`Properties`].
///
/// Follows the state machine in xschem's `get_tok_value` (`src/token.c`):
/// - Pairs are separated by whitespace or `;`.
/// - Values may be double-quoted (`key="value with spaces\nand newlines"`).
/// - Inside a value, `\` escapes the next character (`\"` -> `"`, `\\` -> `\`),
///   and unescaped surrounding `"` quotes are stripped from the stored value.
/// - Preserves exact key insertion order in [`Properties`].
pub fn parse_properties(raw: &str) -> Result<Properties, FormatError> {
    let max = ParseLimits::DEFAULT.max_string_bytes;
    if raw.len() > max {
        return Err(FormatError::InputTooLarge {
            size: raw.len(),
            max,
        });
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Begin,
        Token,
        EndTok,
        Sep,
        Value,
    }

    let mut props = Properties::new();
    let mut state = State::Begin;
    let mut quote = false;
    let mut escape = false;
    let mut key = String::new();
    let mut val = String::new();

    for ch in raw.chars() {
        if ch == '\r' {
            continue;
        }
        let space = is_prop_space(ch);

        // State transitions
        if matches!(state, State::Begin | State::EndTok) && !space && ch != '=' {
            if state == State::EndTok {
                // Previous token had no '=value'; discard bare token and start fresh
                key.clear();
            }
            state = State::Token;
        } else if state == State::Token && space && !quote && !escape {
            state = State::EndTok;
        } else if matches!(state, State::Token | State::EndTok) && ch == '=' {
            state = State::Sep;
        } else if state == State::Sep && !space {
            state = State::Value;
        } else if state == State::Value && space && !quote && !escape {
            props.set(std::mem::take(&mut key), std::mem::take(&mut val));
            state = State::Begin;
        }

        if ch == '"' && !escape {
            quote = !quote;
        }

        match state {
            State::Token => {
                key.push(ch);
            }
            State::Value => {
                if escape || (ch != '\\' && ch != '"') {
                    val.push(ch);
                }
            }
            State::Begin | State::EndTok | State::Sep => {}
        }

        escape = ch == '\\' && !escape;
    }

    if quote {
        return Err(FormatError::Syntax(
            "unclosed double quote '\"' in property string".into(),
        ));
    }

    // Flush any key=value pair that was not followed by whitespace
    match state {
        State::Value | State::Sep => {
            props.set(key, val);
        }
        State::Begin | State::Token | State::EndTok => {}
    }

    Ok(props)
}

/// Format [`Properties`] as the inner text of an xschem `{...}` block.
///
/// Serializes entries in their exact insertion order, quoting values that are empty
/// or contain whitespace, quotes, backslashes, `=`, or `;`.
#[must_use]
pub fn format_properties(props: &Properties) -> String {
    let mut out = String::new();
    for (idx, (key, val)) in props.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        out.push_str(key);
        out.push('=');
        let needs_quotes = val.is_empty()
            || val
                .chars()
                .any(|c| is_prop_space(c) || matches!(c, '"' | '\\' | '='));
        if needs_quotes {
            out.push('"');
            for ch in val.chars() {
                if matches!(ch, '"' | '\\') {
                    out.push('\\');
                }
                out.push(ch);
            }
            out.push('"');
        } else {
            out.push_str(val);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braced_string_round_trips_simple_and_escaped_text() -> Result<(), FormatError> {
        let (parsed, rest) = read_braced_string("  {name=R1 value=1k} trailing")?;
        assert_eq!(parsed, "name=R1 value=1k");
        assert_eq!(rest, " trailing");

        let complex = "tcleval(puts {hello \\ world})";
        let encoded = write_braced_string(complex);
        assert_eq!(encoded, r"{tcleval(puts \{hello \\ world\})}");
        let (decoded, rest) = read_braced_string(&encoded)?;
        assert_eq!(decoded, complex);
        assert_eq!(rest, "");
        Ok(())
    }

    #[test]
    fn braced_string_rejects_missing_or_unclosed_braces() {
        assert!(read_braced_string("no braces here").is_err());
        assert!(read_braced_string("{unterminated").is_err());
        assert!(read_braced_string(r"{escaped end \}").is_err());
    }

    #[test]
    fn properties_parse_res_sym_header_and_pin_attributes() -> Result<(), FormatError> {
        let k_block = "type=resistor\nformat=\"@name @pinlist @value m=@m\"\ntemplate=\"name=R1\nvalue=1k\nfootprint=1206\ndevice=resistor\nm=1\"";
        let props = parse_properties(k_block)?;

        assert_eq!(props.get("type"), Some("resistor"));
        assert_eq!(props.get("format"), Some("@name @pinlist @value m=@m"));
        assert_eq!(
            props.get("template"),
            Some("name=R1\nvalue=1k\nfootprint=1206\ndevice=resistor\nm=1")
        );

        let pin_block = "name=P dir=inout propag=1 pinnumber=1 goto=1";
        let pin_props = parse_properties(pin_block)?;
        assert_eq!(format_properties(&pin_props), pin_block);
        Ok(())
    }

    #[test]
    fn properties_preserve_insertion_order_when_updated() -> Result<(), FormatError> {
        let mut props = parse_properties("name=R1 value=1k m=1")?;
        props.set("value", "4.7k");
        props.set("desc", "pull up");

        assert_eq!(
            format_properties(&props),
            r#"name=R1 value=4.7k m=1 desc="pull up""#
        );
        Ok(())
    }

    #[test]
    fn properties_reject_unclosed_quotes() {
        assert!(parse_properties("name=\"unclosed").is_err());
    }
}
