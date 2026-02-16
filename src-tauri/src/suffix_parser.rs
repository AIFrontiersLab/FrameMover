//! Parse user-provided suffix numbers (comma/space/newline separated) into a set of numeric suffixes.

use std::collections::HashSet;

/// Parses a string of suffix numbers separated by commas, spaces, or newlines.
/// Returns a set of unique positive numbers; invalid tokens are skipped.
pub fn parse_suffixes(input: &str) -> HashSet<u32> {
    let mut set = HashSet::new();
    for token in input.split(|c: char| c == ',' || c.is_whitespace()) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Ok(n) = token.parse::<u32>() {
            set.insert(n);
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_comma_separated() {
        let s = parse_suffixes("7612,7608,7605");
        assert!(s.contains(&7612));
        assert!(s.contains(&7608));
        assert!(s.contains(&7605));
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn test_parse_space_and_newline() {
        let s = parse_suffixes("7612 7608\n7605");
        assert!(s.contains(&7612));
        assert!(s.contains(&7608));
        assert!(s.contains(&7605));
    }

    #[test]
    fn test_parse_dedupe() {
        let s = parse_suffixes("7612, 7612, 7612");
        assert_eq!(s.len(), 1);
        assert!(s.contains(&7612));
    }

    #[test]
    fn test_parse_invalid_skipped() {
        let s = parse_suffixes("7612, abc, 7608, -1, 7605");
        assert!(s.contains(&7612));
        assert!(s.contains(&7608));
        assert!(s.contains(&7605));
        assert_eq!(s.len(), 3);
    }
}
