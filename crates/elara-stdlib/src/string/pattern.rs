//! Small Lua pattern helpers shared by string-library functions.

pub(super) fn has_unsupported_pattern_special(pattern: &[u8]) -> bool {
    pattern.iter().enumerate().any(|(index, byte)| {
        matches!(byte, b'*' | b'+' | b'?' | b'(' | b'[' | b'%' | b'-')
            || (*byte == b'^' && index != 0)
            || (*byte == b'$' && index + 1 != pattern.len())
    })
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
    let end = start.checked_add(pattern.body.len())?;
    if end > haystack.len() || pattern.anchor_end && end != haystack.len() {
        return None;
    }
    pattern_matches(&haystack[start..end], pattern.body).then_some((start, end))
}

fn pattern_matches(window: &[u8], pattern: &[u8]) -> bool {
    window
        .iter()
        .zip(pattern)
        .all(|(subject, pattern)| *pattern == b'.' || subject == pattern)
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
    fn unsupported_specials_exclude_dot_and_valid_anchors() {
        assert!(!has_unsupported_pattern_special(b"a."));
        assert!(!has_unsupported_pattern_special(b"^a.$"));
        assert!(has_unsupported_pattern_special(b"a+"));
        assert!(has_unsupported_pattern_special(b"a^"));
        assert!(has_unsupported_pattern_special(b"a$b"));
    }
}
