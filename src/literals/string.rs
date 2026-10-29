pub(super) fn shorten(source: &[u8]) -> Option<Vec<u8>> {
    let decoded = match source.first()? {
        b'\'' | b'"' => quoted(source)?,
        b'[' => long(source)?,
        _ => return None, // Interpolation stays opaque to literal shortening.
    };
    let mut best = encode_quoted(&decoded, b'"');
    let single = encode_quoted(&decoded, b'\'');
    if single.len() < best.len() {
        best = single;
    }
    if let Some(long) = encode_long(&decoded)
        && long.len() < best.len()
    {
        best = long;
    }
    (best.len() < source.len()).then_some(best)
}

fn quoted(source: &[u8]) -> Option<Vec<u8>> {
    let mut decoded = Vec::new();
    let mut input = source
        .get(1..source.len().checked_sub(1)?)?
        .iter()
        .copied()
        .peekable();
    while let Some(byte) = input.next() {
        if byte != b'\\' {
            decoded.push(byte);
            continue;
        }
        let escaped = input.next()?;
        if escaped.is_ascii_digit() {
            let mut value = u16::from(escaped - b'0');
            for _ in 0..2 {
                if !input.peek().is_some_and(u8::is_ascii_digit) {
                    break;
                }
                value = value * 10 + u16::from(input.next()? - b'0');
            }
            decoded.push(u8::try_from(value).ok()?);
        } else {
            decoded.push(match escaped {
                b'a' => 7,
                b'b' => 8,
                b'f' => 12,
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                b'v' => 11,
                b'\\' | b'\'' | b'"' => escaped,
                // Unknown, version-specific, and physical newline escapes are
                // copied verbatim by declining to transform the whole literal.
                _ => return None,
            });
        }
    }
    Some(decoded)
}

fn long(source: &[u8]) -> Option<Vec<u8>> {
    // Bare CR and LFCR have different semantics in Lua and Luau.
    if source.contains(&b'\r') {
        return None;
    }
    let opener = source.iter().skip(1).take_while(|&&b| b == b'=').count() + 2;
    let content = source.get(opener..source.len().checked_sub(opener)?)?;
    Some(content.strip_prefix(b"\n").unwrap_or(content).to_vec())
}

fn encode_quoted(bytes: &[u8], quote: u8) -> Vec<u8> {
    let mut result = vec![quote];
    for (index, &byte) in bytes.iter().enumerate() {
        match byte {
            b if b == quote || b == b'\\' => result.extend([b'\\', b]),
            7 => result.extend(b"\\a"),
            8 => result.extend(b"\\b"),
            9 => result.extend(b"\\t"),
            10 => result.extend(b"\\n"),
            11 => result.extend(b"\\v"),
            12 => result.extend(b"\\f"),
            13 => result.extend(b"\\r"),
            0..=31 | 127 => {
                // A decimal escape consumes up to three digits. Pad it when
                // the next byte is a digit rather than changing that byte.
                let digits = if bytes.get(index + 1).is_some_and(u8::is_ascii_digit) {
                    format!("\\{byte:03}")
                } else {
                    format!("\\{byte}")
                };
                result.extend(digits.as_bytes());
            }
            _ => result.push(byte),
        }
    }
    result.push(quote);
    result
}

fn encode_long(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.contains(&b'\r') || bytes.contains(&0) {
        return None;
    }
    let mut level = 0;
    let closing = loop {
        let closing = format!("]{}]", "=".repeat(level));
        if !bytes
            .windows(closing.len())
            .any(|window| window == closing.as_bytes())
            && !bytes.ends_with(&closing.as_bytes()[..closing.len() - 1])
        {
            break closing;
        }
        level += 1;
    };
    let mut result = format!("[{}[", "=".repeat(level)).into_bytes();
    // Lua discards exactly one initial newline in a long string.
    if bytes.starts_with(b"\n") {
        result.push(b'\n');
    }
    result.extend(bytes);
    result.extend(closing.as_bytes());
    Some(result)
}
