//! Small Lua pattern helpers shared by string-library functions.

pub(super) fn has_unsupported_pattern_special(pattern: &[u8]) -> bool {
    let mut index = 0;
    while let Some(byte) = pattern.get(index).copied() {
        match byte {
            b'%' => {
                let Some(class) = pattern.get(index + 1).copied() else {
                    return true;
                };
                if matches!(class, b'b' | b'f' | b'0'..=b'9') {
                    return true;
                }
                index += 2;
            }
            b'*' | b'+' | b'?' | b'(' | b'[' | b'-' => return true,
            b'^' if index != 0 => return true,
            b'$' if index + 1 != pattern.len() => return true,
            _ => index += 1,
        }
    }
    false
}

pub(super) fn simple_pattern_find(haystack: &[u8], pattern: &[u8]) -> Option<(usize, usize)> {
    let pattern = ParsedPattern::new(pattern);
    if pattern.body.is_empty() && !pattern.anchor_end {
        return Some((0, 0));
    }
    if pattern.anchor_start {
        return pattern_matches_at(haystack, &pattern, 0);
    }

    (0..=haystack.len()).find_map(|start| pattern_matches_at(haystack, &pattern, start))
}

pub(super) fn is_start_anchored(pattern: &[u8]) -> bool {
    pattern.first() == Some(&b'^')
}

struct ParsedPattern<'a> {
    body: &'a [u8],
    anchor_start: bool,
    anchor_end: bool,
}

impl<'a> ParsedPattern<'a> {
    fn new(pattern: &'a [u8]) -> Self {
        let anchor_start = pattern.first() == Some(&b'^');
        let anchor_end = pattern.last() == Some(&b'$');
        let start = usize::from(anchor_start);
        let end = pattern.len().saturating_sub(usize::from(anchor_end));
        Self {
            body: &pattern[start..end],
            anchor_start,
            anchor_end,
        }
    }
}

fn pattern_matches_at(
    haystack: &[u8],
    pattern: &ParsedPattern<'_>,
    start: usize,
) -> Option<(usize, usize)> {
    let matched_len = pattern_match_len(haystack.get(start..)?, pattern.body)?;
    let end = start.checked_add(matched_len)?;
    if end > haystack.len() || pattern.anchor_end && end != haystack.len() {
        return None;
    }
    Some((start, end))
}

fn pattern_match_len(haystack: &[u8], pattern: &[u8]) -> Option<usize> {
    let mut pattern_index = 0;
    let mut subject_index = 0;
    while let Some(pattern_byte) = pattern.get(pattern_index).copied() {
        let subject_byte = *haystack.get(subject_index)?;
        let matched = if pattern_byte == b'%' {
            pattern_index += 1;
            let class = *pattern.get(pattern_index)?;
            class_matches(subject_byte, class)
        } else {
            pattern_byte == b'.' || pattern_byte == subject_byte
        };
        if !matched {
            return None;
        }
        pattern_index += 1;
        subject_index += 1;
    }
    Some(subject_index)
}

fn class_matches(byte: u8, class: u8) -> bool {
    let matched = match class.to_ascii_lowercase() {
        b'a' => byte.is_ascii_alphabetic(),
        b'c' => byte.is_ascii_control(),
        b'd' => byte.is_ascii_digit(),
        b'g' => byte.is_ascii_graphic(),
        b'l' => byte.is_ascii_lowercase(),
        b'p' => byte.is_ascii_punctuation(),
        b's' => byte.is_ascii_whitespace(),
        b'u' => byte.is_ascii_uppercase(),
        b'w' => byte.is_ascii_alphanumeric(),
        b'x' => byte.is_ascii_hexdigit(),
        b'z' => byte == 0,
        _ => return byte == class,
    };
    if class.is_ascii_lowercase() {
        matched
    } else {
        !matched
    }
}

#[cfg(test)]
mod tests {
    use super::{has_unsupported_pattern_special, simple_pattern_find};

    #[test]
    fn simple_pattern_find_matches_dot_wildcard() {
        assert_eq!(simple_pattern_find(b"abc", b"a."), Some((0, 2)));
        assert_eq!(simple_pattern_find(b"abc", b".c"), Some((1, 3)));
        assert_eq!(simple_pattern_find(b"abc", b"a.."), Some((0, 3)));
        assert_eq!(simple_pattern_find(b"abc", b"z."), None);
    }

    #[test]
    fn simple_pattern_find_honors_anchors() {
        assert_eq!(simple_pattern_find(b"abc", b"^a."), Some((0, 2)));
        assert_eq!(simple_pattern_find(b"abc", b"^b."), None);
        assert_eq!(simple_pattern_find(b"abc", b"b.$"), Some((1, 3)));
        assert_eq!(simple_pattern_find(b"abc", b"b."), Some((1, 3)));
        assert_eq!(simple_pattern_find(b"abc", b"$"), Some((3, 3)));
        assert_eq!(simple_pattern_find(b"abc", b"^$"), None);
        assert_eq!(simple_pattern_find(b"", b"^$"), Some((0, 0)));
    }

    #[test]
    fn simple_pattern_find_matches_percent_classes() {
        assert_eq!(simple_pattern_find(b"abc123", b"%d%d"), Some((3, 5)));
        assert_eq!(simple_pattern_find(b"abc123", b"%D%D"), Some((0, 2)));
        assert_eq!(simple_pattern_find(b"a+b", b"a%+"), Some((0, 2)));
        assert_eq!(simple_pattern_find(b"a.b", b"%.b"), Some((1, 3)));
        assert_eq!(simple_pattern_find(b"a\0b", b"%z"), Some((1, 2)));
    }

    #[test]
    fn unsupported_specials_exclude_dot_valid_anchors_and_classes() {
        assert!(!has_unsupported_pattern_special(b"a."));
        assert!(!has_unsupported_pattern_special(b"^a.$"));
        assert!(!has_unsupported_pattern_special(b"%d%+%."));
        assert!(has_unsupported_pattern_special(b"a+"));
        assert!(has_unsupported_pattern_special(b"a^"));
        assert!(has_unsupported_pattern_special(b"a$b"));
        assert!(has_unsupported_pattern_special(b"%"));
        assert!(has_unsupported_pattern_special(b"%bxy"));
        assert!(has_unsupported_pattern_special(b"%f[a]"));
        assert!(has_unsupported_pattern_special(b"%1"));
    }
}
