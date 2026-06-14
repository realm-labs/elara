//! Small Lua pattern helpers shared by string-library functions.

#[cfg(test)]
pub(super) fn has_unsupported_pattern_special(pattern: &[u8]) -> bool {
    has_unsupported_pattern_special_inner(pattern, false)
}

pub(super) fn has_unsupported_pattern_special_with_captures(pattern: &[u8]) -> bool {
    has_unsupported_pattern_special_inner(pattern, true)
}

fn has_unsupported_pattern_special_inner(pattern: &[u8], allow_captures: bool) -> bool {
    let mut index = 0;
    let mut capture_depth = 0_u32;
    while let Some(byte) = pattern.get(index).copied() {
        match byte {
            b'%' => {
                let Some(class) = pattern.get(index + 1).copied() else {
                    return true;
                };
                if class.is_ascii_digit() {
                    if !allow_captures || class == b'0' {
                        return true;
                    }
                    index += 2;
                    continue;
                }
                if class == b'b' {
                    if pattern.get(index + 2).is_none() || pattern.get(index + 3).is_none() {
                        return true;
                    }
                    index += 4;
                } else if class == b'f' {
                    if pattern.get(index + 2) != Some(&b'[') {
                        return true;
                    }
                    let Some(end) = bracket_end(pattern, index + 2) else {
                        return true;
                    };
                    index = end + 1;
                } else {
                    index += 2;
                }
            }
            b'[' => {
                let Some(end) = bracket_end(pattern, index) else {
                    return true;
                };
                index = end + 1;
            }
            b'(' if allow_captures => {
                capture_depth += 1;
                index += 1;
            }
            b')' if allow_captures => {
                let Some(depth) = capture_depth.checked_sub(1) else {
                    return true;
                };
                capture_depth = depth;
                index += 1;
            }
            b'(' | b')' => return true,
            b'^' if index != 0 => return true,
            b'$' if index + 1 != pattern.len() => return true,
            _ => index += 1,
        }
    }
    capture_depth != 0
}

#[cfg(test)]
pub(super) fn simple_pattern_find(haystack: &[u8], pattern: &[u8]) -> Option<(usize, usize)> {
    simple_pattern_find_from(haystack, pattern, 0)
}

#[cfg(test)]
pub(super) fn simple_pattern_find_from(
    haystack: &[u8],
    pattern: &[u8],
    start: usize,
) -> Option<(usize, usize)> {
    simple_pattern_match_from(haystack, pattern, start).map(|match_| (match_.start, match_.end))
}

pub(super) fn simple_pattern_match_from(
    haystack: &[u8],
    pattern: &[u8],
    start: usize,
) -> Option<PatternMatch> {
    simple_pattern_match_from_with_anchor_mode(haystack, pattern, start, true)
}

pub(super) fn simple_pattern_match_from_without_start_anchor(
    haystack: &[u8],
    pattern: &[u8],
    start: usize,
) -> Option<PatternMatch> {
    simple_pattern_match_from_with_anchor_mode(haystack, pattern, start, false)
}

fn simple_pattern_match_from_with_anchor_mode(
    haystack: &[u8],
    pattern: &[u8],
    start: usize,
    honor_start_anchor: bool,
) -> Option<PatternMatch> {
    let pattern = ParsedPattern::new(pattern, honor_start_anchor);
    if pattern.body.is_empty() && !pattern.anchor_end {
        return (start <= haystack.len()).then_some(PatternMatch {
            start,
            end: start,
            captures: Vec::new(),
        });
    }
    if start > haystack.len() {
        return None;
    }
    if pattern.anchor_start {
        return pattern_matches_at(haystack, &pattern, start);
    }

    (start..=haystack.len()).find_map(|start| pattern_matches_at(haystack, &pattern, start))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PatternMatch {
    pub start: usize,
    pub end: usize,
    pub captures: Vec<PatternCapture>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PatternCapture {
    String { start: usize, end: usize },
    Position(usize),
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
    fn new(pattern: &'a [u8], honor_start_anchor: bool) -> Self {
        let anchor_start = honor_start_anchor && pattern.first() == Some(&b'^');
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
) -> Option<PatternMatch> {
    let state = pattern_match_end(haystack, pattern.body, start)?;
    if state.end > haystack.len() || pattern.anchor_end && state.end != haystack.len() {
        return None;
    }
    Some(PatternMatch {
        start,
        end: state.end,
        captures: state
            .captures
            .into_iter()
            .map(|capture| match capture {
                Capture::Closed(start, end) => Some(PatternCapture::String { start, end }),
                Capture::Position(position) => Some(PatternCapture::Position(position)),
                Capture::Open(_) => None,
            })
            .collect::<Option<_>>()?,
    })
}

fn pattern_match_end(haystack: &[u8], pattern: &[u8], start: usize) -> Option<MatchState> {
    match_from(haystack, pattern, 0, start, Vec::new())
}

#[derive(Clone)]
struct MatchState {
    end: usize,
    captures: Vec<Capture>,
}

#[derive(Clone)]
enum Capture {
    Open(usize),
    Closed(usize, usize),
    Position(usize),
}

fn match_from(
    haystack: &[u8],
    pattern: &[u8],
    pattern_index: usize,
    subject_index: usize,
    captures: Vec<Capture>,
) -> Option<MatchState> {
    if pattern_index >= pattern.len() {
        return Some(MatchState {
            end: subject_index,
            captures,
        });
    }

    if pattern[pattern_index] == b'(' && pattern.get(pattern_index + 1) == Some(&b')') {
        let mut captures = captures;
        captures.push(Capture::Position(subject_index));
        return match_from(
            haystack,
            pattern,
            pattern_index + 2,
            subject_index,
            captures,
        );
    }

    if pattern[pattern_index] == b'(' {
        let mut captures = captures;
        captures.push(Capture::Open(subject_index));
        return match_from(
            haystack,
            pattern,
            pattern_index + 1,
            subject_index,
            captures,
        );
    }

    if pattern[pattern_index] == b')' {
        let mut captures = captures;
        let position = captures
            .iter()
            .rposition(|capture| matches!(capture, Capture::Open(_)))?;
        let Capture::Open(start) = captures[position] else {
            return None;
        };
        captures[position] = Capture::Closed(start, subject_index);
        return match_from(
            haystack,
            pattern,
            pattern_index + 1,
            subject_index,
            captures,
        );
    }

    let atom_end = atom_end(pattern, pattern_index)?;
    match pattern.get(atom_end).copied() {
        Some(b'?') => match_optional(
            haystack,
            pattern,
            pattern_index,
            atom_end,
            subject_index,
            captures,
        ),
        Some(b'*') => match_greedy_repeat(
            haystack,
            pattern,
            pattern_index,
            atom_end + 1,
            subject_index,
            false,
            captures,
        ),
        Some(b'+') => match_greedy_repeat(
            haystack,
            pattern,
            pattern_index,
            atom_end + 1,
            subject_index,
            true,
            captures,
        ),
        Some(b'-') => match_minimal_repeat(
            haystack,
            pattern,
            pattern_index,
            atom_end + 1,
            subject_index,
            captures,
        ),
        _ => {
            let consumed = atom_match_len(
                haystack,
                pattern,
                pattern_index,
                atom_end,
                subject_index,
                &captures,
            )?;
            match_from(
                haystack,
                pattern,
                atom_end,
                subject_index + consumed,
                captures,
            )
        }
    }
}

fn atom_end(pattern: &[u8], pattern_index: usize) -> Option<usize> {
    match pattern.get(pattern_index).copied()? {
        b'%' if pattern.get(pattern_index + 1) == Some(&b'b') => {
            pattern.get(pattern_index + 3).map(|_| pattern_index + 4)
        }
        b'%' if pattern.get(pattern_index + 1) == Some(&b'f') => {
            if pattern.get(pattern_index + 2) == Some(&b'[') {
                bracket_end(pattern, pattern_index + 2).map(|end| end + 1)
            } else {
                None
            }
        }
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
    captures: Vec<Capture>,
) -> Option<MatchState> {
    if let Some(consumed) = atom_match_len(
        haystack,
        pattern,
        pattern_index,
        atom_end,
        subject_index,
        &captures,
    ) && let Some(end) = match_from(
        haystack,
        pattern,
        atom_end + 1,
        subject_index + consumed,
        captures.clone(),
    ) {
        return Some(end);
    }
    match_from(haystack, pattern, atom_end + 1, subject_index, captures)
}

fn match_greedy_repeat(
    haystack: &[u8],
    pattern: &[u8],
    pattern_index: usize,
    rest_index: usize,
    subject_index: usize,
    require_one: bool,
    captures: Vec<Capture>,
) -> Option<MatchState> {
    let mut end = subject_index;
    let mut candidates = vec![subject_index];
    while let Some(consumed) = atom_match_len(
        haystack,
        pattern,
        pattern_index,
        rest_index - 1,
        end,
        &captures,
    ) {
        if consumed == 0 {
            break;
        }
        end += consumed;
        candidates.push(end);
    }
    if require_one && end == subject_index {
        return None;
    }
    for candidate in candidates.into_iter().rev() {
        if let Some(end) = match_from(haystack, pattern, rest_index, candidate, captures.clone()) {
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
    captures: Vec<Capture>,
) -> Option<MatchState> {
    let mut candidate = subject_index;
    loop {
        if let Some(end) = match_from(haystack, pattern, rest_index, candidate, captures.clone()) {
            return Some(end);
        }
        let consumed = atom_match_len(
            haystack,
            pattern,
            pattern_index,
            rest_index - 1,
            candidate,
            &captures,
        )?;
        if consumed == 0 {
            return None;
        }
        candidate += consumed;
    }
}

fn atom_match_len(
    haystack: &[u8],
    pattern: &[u8],
    pattern_index: usize,
    atom_end: usize,
    subject_index: usize,
    captures: &[Capture],
) -> Option<usize> {
    let matched = match pattern[pattern_index] {
        b'%' if pattern.get(pattern_index + 1) == Some(&b'b') => {
            return balanced_match_len(
                haystack,
                subject_index,
                pattern[pattern_index + 2],
                pattern[pattern_index + 3],
            );
        }
        b'%' if pattern.get(pattern_index + 1) == Some(&b'f') => {
            return frontier_match_len(haystack, pattern, pattern_index, atom_end, subject_index);
        }
        b'%' if pattern
            .get(pattern_index + 1)
            .is_some_and(u8::is_ascii_digit) =>
        {
            return backreference_match_len(
                haystack,
                subject_index,
                pattern[pattern_index + 1],
                captures,
            );
        }
        _ => {
            let &subject_byte = haystack.get(subject_index)?;
            match pattern[pattern_index] {
                b'%' => pattern
                    .get(pattern_index + 1)
                    .is_some_and(|class| class_matches(subject_byte, *class)),
                b'[' => bracket_class_matches(subject_byte, &pattern[pattern_index..atom_end]),
                pattern_byte => pattern_byte == b'.' || pattern_byte == subject_byte,
            }
        }
    };
    matched.then_some(1)
}

fn backreference_match_len(
    haystack: &[u8],
    subject_index: usize,
    digit: u8,
    captures: &[Capture],
) -> Option<usize> {
    let capture_index = usize::from(digit.checked_sub(b'1')?);
    let Capture::Closed(start, end) = captures.get(capture_index)? else {
        return None;
    };
    let captured = &haystack[*start..*end];
    haystack[subject_index..]
        .starts_with(captured)
        .then_some(captured.len())
}

fn balanced_match_len(haystack: &[u8], subject_index: usize, open: u8, close: u8) -> Option<usize> {
    if haystack.get(subject_index) != Some(&open) {
        return None;
    }
    let mut depth = 1_u32;
    for (offset, byte) in haystack[subject_index + 1..].iter().copied().enumerate() {
        if byte == close {
            depth -= 1;
            if depth == 0 {
                return Some(offset + 2);
            }
        } else if byte == open {
            depth += 1;
        }
    }
    None
}

fn frontier_match_len(
    haystack: &[u8],
    pattern: &[u8],
    pattern_index: usize,
    atom_end: usize,
    subject_index: usize,
) -> Option<usize> {
    let class = &pattern[(pattern_index + 2)..atom_end];
    let previous = if subject_index == 0 {
        0
    } else {
        haystack[subject_index - 1]
    };
    let current = haystack.get(subject_index).copied().unwrap_or(0);
    (!bracket_class_matches(previous, class) && bracket_class_matches(current, class)).then_some(0)
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
    use super::{
        PatternCapture, has_unsupported_pattern_special,
        has_unsupported_pattern_special_with_captures, simple_pattern_find,
        simple_pattern_find_from, simple_pattern_match_from,
        simple_pattern_match_from_without_start_anchor,
    };

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
    fn simple_pattern_match_can_treat_start_anchor_as_literal() {
        let match_ = simple_pattern_match_from_without_start_anchor(b"a^b ^c", b"^.", 0)
            .expect("literal caret should match");

        assert_eq!(match_.start, 1);
        assert_eq!(match_.end, 3);
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
    fn simple_pattern_find_matches_balanced_delimiters() {
        assert_eq!(simple_pattern_find(b"a(b(c)d)e", b"%b()"), Some((1, 8)));
        assert_eq!(simple_pattern_find(b"{a}{b}", b"%b{}"), Some((0, 3)));
        assert_eq!(simple_pattern_find(b"(abc", b"%b()"), None);
    }

    #[test]
    fn simple_pattern_find_matches_frontiers() {
        assert_eq!(simple_pattern_find(b"abc 123", b"%f[%d]%d+"), Some((4, 7)));
        assert_eq!(simple_pattern_find(b"abc", b"%f[%a]a"), Some((0, 1)));
        assert_eq!(simple_pattern_find(b"abc", b"%f[%z]"), Some((3, 3)));
        assert_eq!(simple_pattern_find(b"abc", b"%f[%d]"), None);
    }

    #[test]
    fn simple_pattern_find_from_preserves_frontier_context() {
        assert_eq!(
            simple_pattern_find_from(b"abc def", b"%f[%a]%a+", 3),
            Some((4, 7))
        );
        assert_eq!(simple_pattern_find_from(b"abc", b"%f[%a]%a+", 1), None);
    }

    #[test]
    fn simple_pattern_match_from_records_captures() {
        let match_ =
            simple_pattern_match_from(b"abc123", b"(%a+)(%d+)", 0).expect("captures should match");

        assert_eq!(match_.start, 0);
        assert_eq!(match_.end, 6);
        assert_eq!(
            match_.captures,
            vec![
                PatternCapture::String { start: 0, end: 3 },
                PatternCapture::String { start: 3, end: 6 }
            ]
        );
    }

    #[test]
    fn simple_pattern_match_from_records_position_captures() {
        let match_ =
            simple_pattern_match_from(b"flaaap", b"()aa()", 0).expect("captures should match");

        assert_eq!(match_.start, 2);
        assert_eq!(match_.end, 4);
        assert_eq!(
            match_.captures,
            vec![PatternCapture::Position(2), PatternCapture::Position(4)]
        );
    }

    #[test]
    fn simple_pattern_find_matches_capture_backreferences() {
        assert_eq!(
            simple_pattern_find(b"alo alx 123 b\0o b\0o", b"(..*) %1"),
            Some((12, 19))
        );
        assert_eq!(simple_pattern_find(b"==========", b"^([=]*)=%1$"), None);
        assert_eq!(simple_pattern_find(b"=======", b"^(=*)=%1$"), Some((0, 7)));
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
        assert!(!has_unsupported_pattern_special(b"%bxy"));
        assert!(has_unsupported_pattern_special(b"%bx"));
        assert!(!has_unsupported_pattern_special(b"%f[a]"));
        assert!(has_unsupported_pattern_special(b"%fa"));
        assert!(has_unsupported_pattern_special(b"%1"));
        assert!(has_unsupported_pattern_special_with_captures(b"%0"));
        assert!(!has_unsupported_pattern_special_with_captures(
            b"(%a+)(%d+)"
        ));
        assert!(!has_unsupported_pattern_special_with_captures(b"(%a+) %1"));
        assert!(has_unsupported_pattern_special_with_captures(b"(%a+"));
    }
}
