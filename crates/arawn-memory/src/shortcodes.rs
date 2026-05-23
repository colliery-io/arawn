//! Entity shortcode compression — replaces repeated entity names with
//! short codes to save tokens in L1 output.
//!
//! Applied only to rendered output, never to storage.

use std::collections::HashMap;

/// Scan text for repeated entity-like names and replace with shortcodes.
///
/// Names appearing `min_occurrences` or more times get a 2-3 char code
/// generated from first letters of each word. A legend is prepended.
///
/// Entity names are provided externally (from the KB) so we know exactly
/// which strings to look for, rather than guessing from the text.
pub fn apply_shortcodes(text: &str, entity_names: &[String], min_occurrences: usize) -> String {
    if entity_names.is_empty() || min_occurrences < 2 {
        return text.to_string();
    }

    // Count occurrences of each entity name (case-insensitive)
    let text_lower = text.to_lowercase();
    let mut candidates: Vec<(&String, usize)> = entity_names
        .iter()
        .filter_map(|name| {
            let count = count_occurrences(&text_lower, &name.to_lowercase());
            if count >= min_occurrences {
                Some((name, count))
            } else {
                None
            }
        })
        .collect();

    if candidates.is_empty() {
        return text.to_string();
    }

    // Sort by occurrence count descending (most repeated = most savings)
    candidates.sort_by(|a, b| b.1.cmp(&a.1));

    // Generate codes, handling collisions
    let mut used_codes: HashMap<String, String> = HashMap::new();
    let mut mappings: Vec<(String, String)> = Vec::new();

    for (name, _count) in &candidates {
        let mut code = generate_code(name);

        // Handle collision
        if used_codes.contains_key(&code) {
            let mut suffix = 2;
            loop {
                let candidate = format!("{code}{suffix}");
                if !used_codes.contains_key(&candidate) {
                    code = candidate;
                    break;
                }
                suffix += 1;
            }
        }

        used_codes.insert(code.clone(), name.to_string());
        mappings.push((name.to_string(), code));
    }

    // Build legend
    let legend_parts: Vec<String> = mappings
        .iter()
        .map(|(name, code)| format!("{code}={name}"))
        .collect();
    let legend = format!("({})\n", legend_parts.join(", "));

    // Replace in text (case-preserving: replace exact matches)
    let mut result = text.to_string();
    for (name, code) in &mappings {
        result = result.replace(name.as_str(), code);
    }

    format!("{legend}{result}")
}

/// Count non-overlapping occurrences of needle in haystack.
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.matches(needle).count()
}

/// Generate a shortcode from a name: first letter of each word, uppercased.
/// "arawn-engine" -> "AE", "Dylan Storey" -> "DS", "memory" -> "M"
fn generate_code(name: &str) -> String {
    let code: String = name
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|w| !w.is_empty())
        .map(|w| w.chars().next().unwrap_or('?'))
        .collect::<String>()
        .to_uppercase();

    if code.is_empty() {
        "X".to_string()
    } else {
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compresses_repeated_names() {
        let text = "arawn-engine uses tracing. The arawn-engine crate handles queries. arawn-engine is fast.";
        let names = vec!["arawn-engine".to_string()];
        let result = apply_shortcodes(text, &names, 2);

        assert!(result.contains("AE=arawn-engine"));
        assert!(result.contains("AE uses tracing"));
        assert!(!result.contains("arawn-engine uses"));
    }

    #[test]
    fn skips_single_occurrence() {
        let text = "arawn-engine is here. Nothing else repeats.";
        let names = vec!["arawn-engine".to_string()];
        let result = apply_shortcodes(text, &names, 2);

        // Should be unchanged (no legend, no replacement)
        assert_eq!(result, text);
    }

    #[test]
    fn handles_collision() {
        let text = "arawn-engine and arawn-embed both start with A. arawn-engine again. arawn-embed again.";
        let names = vec!["arawn-engine".to_string(), "arawn-embed".to_string()];
        let result = apply_shortcodes(text, &names, 2);

        // Both should be compressed, one with a suffix
        assert!(result.contains("AE="));
        assert!(result.contains("AE2=") || result.contains("AE=arawn-embed"));
    }

    #[test]
    fn empty_names_returns_unchanged() {
        let text = "some text";
        let result = apply_shortcodes(text, &[], 2);
        assert_eq!(result, text);
    }

    #[test]
    fn multi_word_name() {
        let text = "Dylan Storey wrote the code. Dylan Storey reviewed it. Dylan Storey approved.";
        let names = vec!["Dylan Storey".to_string()];
        let result = apply_shortcodes(text, &names, 2);

        assert!(result.contains("DS=Dylan Storey"));
        assert!(result.contains("DS wrote"));
    }
}
