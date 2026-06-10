//! Small Lua pattern helpers shared by string-library functions.

pub(super) fn has_unsupported_pattern_special(pattern: &[u8]) -> bool {
    pattern.iter().any(|byte| {
        matches!(
            byte,
            b'^' | b'$' | b'*' | b'+' | b'?' | b'(' | b'[' | b'%' | b'-'
        )
    })
}

pub(super) fn simple_pattern_find(haystack: &[u8], pattern: &[u8]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return Some((0, 0));
    }
    if pattern.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(pattern.len())
        .position(|window| pattern_matches(window, pattern))
        .map(|start| (start, start + pattern.len()))
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
    fn unsupported_specials_exclude_dot() {
        assert!(!has_unsupported_pattern_special(b"a."));
        assert!(has_unsupported_pattern_special(b"a+"));
    }
}
