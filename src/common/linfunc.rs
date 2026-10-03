//! Port of src/common/linfunc.cpp

pub fn in_place_lower_case(s: &mut String) {
    s.make_ascii_lowercase();
}

pub fn lowercase(mut s: String) -> String {
    in_place_lower_case(&mut s);
    s
}

/// Compares up to the end of `a` only, like the C++ loop (a prefix of `b` compares equal).
pub fn cstr_ci_equals(a: &str, b: &str) -> bool {
    if std::ptr::eq(a, b) {
        return true;
    }

    let pb = b.as_bytes();
    for (i, &ca) in a.as_bytes().iter().enumerate() {
        let cb = pb.get(i).copied().unwrap_or(0);
        if ca.to_ascii_lowercase() != cb.to_ascii_lowercase() {
            return false;
        }
    }

    true
}

pub fn tokenize(text: &str, delim: char, mut maxsplit: usize) -> Vec<&str> {
    if text.is_empty() {
        return vec![text];
    }

    let mut tokens = Vec::new();
    let mut start = 0usize;

    while start < text.len() {
        if maxsplit == 0 {
            tokens.push(&text[start..]);
            break;
        }

        let Some(off) = text[start..].find(delim) else {
            tokens.push(&text[start..]);
            break;
        };
        let end = start + off;

        tokens.push(&text[start..end]);
        start = end + 1;
        maxsplit = maxsplit.wrapping_sub(1);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_matches_cpp() {
        assert_eq!(tokenize("", ',', usize::MAX), vec![""]);
        assert_eq!(tokenize("a,b,,c", ',', usize::MAX), vec!["a", "b", "", "c"]);
        assert_eq!(tokenize("a,b,", ',', usize::MAX), vec!["a", "b"]);
        assert_eq!(tokenize("a,b,c", ',', 1), vec!["a", "b,c"]);
        assert!(cstr_ci_equals("AbC", "abcdef"));
        assert!(!cstr_ci_equals("abcd", "abc"));
    }
}
