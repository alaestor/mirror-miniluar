/// Shorten decimal spelling without converting through a machine number. This
/// preserves integer/float syntax, overflow, subnormals, and unary negative zero.
pub(super) fn shorten(source: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(source).ok()?;
    let clean = text.replace('_', "");
    if clean.starts_with('0')
        && clean
            .as_bytes()
            .get(1)
            .is_some_and(|b| matches!(b, b'x' | b'X' | b'b' | b'B'))
    {
        return radix(&clean).filter(|candidate| candidate.len() < source.len());
    }
    let exponent_at = clean.find(['e', 'E']);
    let (mantissa, exponent) = exponent_at.map_or((clean.as_str(), None), |at| {
        (&clean[..at], Some(&clean[at + 1..]))
    });
    let (whole, fraction) = mantissa
        .split_once('.')
        .map_or((mantissa, None), |(whole, part)| (whole, Some(part)));
    if !digits(whole)
        || fraction.is_some_and(|s| !digits(s))
        || (whole.is_empty() && fraction.is_none_or(str::is_empty))
    {
        return None;
    }
    let whole = whole.trim_start_matches('0');
    let fraction = fraction.map(|s| s.trim_end_matches('0'));
    let mut result = String::new();
    if !whole.is_empty() {
        result.push_str(whole);
    } else if fraction.is_none_or(str::is_empty) {
        result.push('0');
    }
    if let Some(fraction) = fraction {
        // An exponent already makes this a float. Without one, retain the dot
        // even when all fractional digits vanished, so 1.0 never becomes 1.
        if !fraction.is_empty() || exponent.is_none() {
            result.push('.');
            result.push_str(fraction);
        }
    }
    if let Some(exponent) = exponent {
        let negative = exponent.starts_with('-');
        let unsigned = exponent.strip_prefix(['-', '+']).unwrap_or(exponent);
        if unsigned.is_empty() || !digits(unsigned) {
            return None;
        }
        let significant = unsigned.trim_start_matches('0');
        result.push('e');
        if significant.is_empty() {
            result.push('0');
        } else {
            if negative {
                result.push('-');
            }
            result.push_str(significant);
        }
    }
    (result.len() < source.len()).then(|| result.into_bytes())
}

fn digits(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii_digit())
}

fn radix(text: &str) -> Option<Vec<u8>> {
    let binary = text
        .as_bytes()
        .get(1)
        .is_some_and(|b| matches!(b, b'b' | b'B'));
    let body = &text[2..];
    // Leave hexadecimal floats alone. Removing separators from integer forms
    // is safe in Luau, without changing Lua's hexadecimal overflow behavior.
    if body.is_empty()
        || !body.bytes().all(|b| {
            if binary {
                matches!(b, b'0' | b'1')
            } else {
                b.is_ascii_hexdigit()
            }
        })
    {
        return None;
    }
    Some(text.as_bytes().to_vec())
}
