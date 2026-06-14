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
            b'[' => {
                let Some(end) = bracket_end(pattern, index) else {
                    return true;
                };
                index = end + 1;
            }
            b'(' => return true,
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
    match_from(haystack, pattern, 0, 0)
}

fn match_from(
    haystack: &[u8],
    pattern: &[u8],
    pattern_index: usize,
    subject_index: usize,
) -> Option<usize> {
    if pattern_index >= pattern.len() {
        return Some(subject_index);
    }

    let atom_end = atom_end(pattern, pattern_index)?;
    match pattern.get(atom_end).copied() {
        Some(b'?') => match_optional(haystack, pattern, pattern_index, atom_end, subject_index),
        Some(b'*') => match_greedy_repeat(
            haystack,
            pattern,
            pattern_index,
            atom_end + 1,
            subject_index,
            false,
        ),
        Some(b'+') => match_greedy_repeat(
            haystack,
            pattern,
            pattern_index,
            atom_end + 1,
            subject_index,
            true,
        ),
        Some(b'-') => match_minimal_repeat(
            haystack,
            pattern,
            pattern_index,
            atom_end + 1,
            subject_index,
        ),
        _ if atom_matches(haystack, pattern, pattern_index, atom_end, subject_index) => {
            match_from(haystack, pattern, atom_end, subject_index + 1)
        }
        _ => None,
    }
}

fn atom_end(pattern: &[u8], pattern_index: usize) -> Option<usize> {
    match pattern.get(pattern_index).copied()? {
        b'%' => pattern.get(pattern_index + 1).map(|_| pattern_index + 2),
        b'[' => bracket_end(pattern, pattern_index).map(|end| end + 1),
        _ => Some(pattern_index + 1),
    }
}

fn match_optional(
    haystack: &[u8],
    pattern: &[u8],
    pattern_index: usize,
    atom_end: usize,
    subject_index: usize,
) -> Option<usize> {
    if atom_matches(haystack, pattern, pattern_index, atom_end, subject_index)
        && let Some(end) = match_from(haystack, pattern, atom_end + 1, subject_index + 1)
    {
        return Some(end);
    }
    match_from(haystack, pattern, atom_end + 1, subject_index)
}

fn match_greedy_repeat(
    haystack: &[u8],
    pattern: &[u8],
    pattern_index: usize,
    rest_index: usize,
    subject_index: usize,
    require_one: bool,
) -> Option<usize> {
    let mut end = subject_index;
    while atom_matches(haystack, pattern, pattern_index, rest_index - 1, end) {
        end += 1;
    }
    if require_one && end == subject_index {
        return None;
    }
    for candidate in (subject_index..=end).rev() {
        if let Some(end) = match_from(haystack, pattern, rest_index, candidate) {
            return Some(end);
        }
    }
    None
}

fn match_minimal_repeat(
    haystack: &[u8],
    pattern: &[u8],
    pattern_index: usize,
    rest_index: usize,
    subject_index: usize,
) -> Option<usize> {
    let mut candidate = subject_index;
    loop {
        if let Some(end) = match_from(haystack, pattern, rest_index, candidate) {
            return Some(end);
        }
        if !atom_matches(haystack, pattern, pattern_index, rest_index - 1, candidate) {
            return None;
        }
        candidate += 1;
    }
}

fn atom_matches(
    haystack: &[u8],
    pattern: &[u8],
    pattern_index: usize,
    atom_end: usize,
    subject_index: usize,
) -> bool {
    let Some(&subject_byte) = haystack.get(subject_index) else {
        return false;
    };
    match pattern[pattern_index] {
        b'%' => pattern
            .get(pattern_index + 1)
            .is_some_and(|class| class_matches(subject_byte, *class)),
        b'[' => bracket_class_matches(subject_byte, &pattern[pattern_index..atom_end]),
        pattern_byte => pattern_byte == b'.' || pattern_byte == subject_byte,
    }
}

fn bracket_end(pattern: &[u8], start: usize) -> Option<usize> {
    debug_assert_eq!(pattern.get(start), Some(&b'['));
    let mut index = start + 1;
    if pattern.get(index) == Some(&b'^') {
        index += 1;
    }
    while index < pattern.len() {
        if pattern[index] == b'%' && index + 1 < pattern.len() {
            index += 2;
        } else if pattern[index] == b']' {
            return Some(index);
        } else {
            index += 1;
        }
    }
    None
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

fn bracket_class_matches(byte: u8, class: &[u8]) -> bool {
    let negated = class.get(1) == Some(&b'^');
    let end = class.len().saturating_sub(1);
    let mut index = if negated { 2 } else { 1 };
    let matched = loop {
        if index >= end {
            break false;
        }
        if class[index] == b'%' && index + 1 < end {
            index += 1;
            if class_matches(byte, class[index]) {
                break true;
            }
            index += 1;
        } else if index + 2 < end && class[index + 1] == b'-' {
            if class[index] <= byte && byte <= class[index + 2] {
                break true;
            }
            index += 3;
        } else if class[index] == byte {
            break true;
        } else {
            index += 1;
        }
    };
    if negated { !matched } else { matched }
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
    fn simple_pattern_find_matches_bracket_classes() {
        assert_eq!(simple_pattern_find(b"abc123", b"[0-9][0-9]"), Some((3, 5)));
        assert_eq!(simple_pattern_find(b"abc123", b"[^a-c][0-9]"), Some((3, 5)));
        assert_eq!(simple_pattern_find(b"abc123", b"[%a][%d]"), Some((2, 4)));
        assert_eq!(simple_pattern_find(b"a]b", b"[%]]"), Some((1, 2)));
        assert_eq!(simple_pattern_find(b"abc", b"[x-z]"), None);
    }

    #[test]
    fn simple_pattern_find_matches_quantifiers() {
        assert_eq!(simple_pattern_find(b"aaab", b"a+b"), Some((0, 4)));
        assert_eq!(simple_pattern_find(b"aaab", b"a*b"), Some((0, 4)));
        assert_eq!(simple_pattern_find(b"b", b"a*b"), Some((0, 1)));
        assert_eq!(simple_pattern_find(b"ab", b"ac?b"), Some((0, 2)));
        assert_eq!(simple_pattern_find(b"abcb", b"a.-b"), Some((0, 2)));
        assert_eq!(simple_pattern_find(b"abcb", b"a.*b"), Some((0, 4)));
        assert_eq!(simple_pattern_find(b"bbb", b"a+"), None);
    }

    #[test]
    fn unsupported_specials_exclude_dot_valid_anchors_and_classes() {
        assert!(!has_unsupported_pattern_special(b"a."));
        assert!(!has_unsupported_pattern_special(b"^a.$"));
        assert!(!has_unsupported_pattern_special(b"%d%+%."));
        assert!(!has_unsupported_pattern_special(b"[a-z]%d"));
        assert!(!has_unsupported_pattern_special(b"a+"));
        assert!(!has_unsupported_pattern_special(b"a*b?c-"));
        assert!(has_unsupported_pattern_special(b"(a)"));
        assert!(has_unsupported_pattern_special(b"a^"));
        assert!(has_unsupported_pattern_special(b"a$b"));
        assert!(has_unsupported_pattern_special(b"%"));
        assert!(has_unsupported_pattern_special(b"[abc"));
        assert!(has_unsupported_pattern_special(b"%bxy"));
        assert!(has_unsupported_pattern_special(b"%f[a]"));
        assert!(has_unsupported_pattern_special(b"%1"));
    }
}
