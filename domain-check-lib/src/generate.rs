//! Domain name generation engine.
//!
//! This module generates base domain names from patterns and prefix/suffix permutations.
//! It produces base names only — TLD expansion is handled separately by `expand_domain_inputs`.
//!
//! # Pattern Syntax
//!
//! - `\w` — lowercase letter (a-z) or hyphen (excluded at first/last position)
//! - `\d` — digit (0-9)
//! - `?`  — any of the above (letter, digit, or hyphen)
//! - `\\` — literal backslash
//! - Any other character — literal
//!
//! # Examples
//!
//! ```
//! use domain_check_lib::generate::{expand_pattern, apply_affixes, generate_names};
//! use domain_check_lib::GenerateConfig;
//!
//! // Pattern expansion
//! let names = expand_pattern("app\\d\\d").unwrap();
//! assert_eq!(names.len(), 100); // app00..app99
//!
//! // Prefix/suffix
//! let base = vec!["cloud".to_string()];
//! let affixed: Vec<_> = apply_affixes(&base, &["get".to_string()], &["ly".to_string()], true).collect();
//! assert!(affixed.contains(&"getcloudly".to_string()));
//! assert!(affixed.contains(&"cloud".to_string())); // bare included
//! ```

use crate::error::DomainCheckError;
use crate::types::{GenerateConfig, GenerationResult};
use crate::utils::is_valid_base_name;

/// A single slot in a parsed pattern — either a fixed character or a set of possibilities.
#[derive(Debug, Clone)]
enum Slot {
    Literal(char),
    Charset(Vec<char>),
}

/// Characters for `\w`: a-z plus hyphen.
fn word_chars() -> Vec<char> {
    let mut chars: Vec<char> = ('a'..='z').collect();
    chars.push('-');
    chars
}

/// Characters for `\d`: 0-9.
fn digit_chars() -> Vec<char> {
    ('0'..='9').collect()
}

/// Characters for `?`: union of `\w` and `\d`.
fn any_chars() -> Vec<char> {
    let mut chars = word_chars();
    chars.extend(digit_chars());
    chars
}

/// Parse a pattern string into a sequence of slots.
///
/// Returns an error for invalid escape sequences.
fn parse_pattern(pattern: &str) -> Result<Vec<Slot>, DomainCheckError> {
    if pattern.is_empty() {
        return Err(DomainCheckError::invalid_pattern(
            pattern,
            "pattern cannot be empty",
        ));
    }

    let mut slots = Vec::new();
    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.next() {
                Some('w') => slots.push(Slot::Charset(word_chars())),
                Some('d') => slots.push(Slot::Charset(digit_chars())),
                Some('\\') => slots.push(Slot::Literal('\\')),
                Some(other) => {
                    return Err(DomainCheckError::invalid_pattern(
                        pattern,
                        format!("unknown escape sequence '\\{}'", other),
                    ));
                }
                None => {
                    return Err(DomainCheckError::invalid_pattern(
                        pattern,
                        "trailing backslash",
                    ));
                }
            },
            '?' => slots.push(Slot::Charset(any_chars())),
            _ => slots.push(Slot::Literal(ch)),
        }
    }

    Ok(slots)
}

/// Estimate how many names a pattern will produce before filtering.
///
/// This is the raw Cartesian product size — actual count may be lower
/// after `is_valid_base_name` filtering removes names with leading/trailing
/// hyphens or names that are too short.
pub fn estimate_pattern_count(pattern: &str) -> Result<usize, DomainCheckError> {
    let slots = parse_pattern(pattern)?;
    let mut count: usize = 1;
    for slot in &slots {
        let slot_size = match slot {
            Slot::Literal(_) => 1,
            Slot::Charset(chars) => chars.len(),
        };
        count = count.saturating_mul(slot_size);
    }
    Ok(count)
}

/// Expand a pattern into all matching base domain names.
///
/// Uses an odometer-style algorithm: iterates through all combinations
/// by treating each charset slot as a digit in a mixed-radix number.
/// Names are filtered through `is_valid_base_name` (removes leading/trailing
/// hyphens, names shorter than 2 chars, etc.).
pub fn expand_pattern(pattern: &str) -> Result<Vec<String>, DomainCheckError> {
    let slots = parse_pattern(pattern)?;

    // Build the list of char options per slot
    let options: Vec<Vec<char>> = slots
        .iter()
        .map(|s| match s {
            Slot::Literal(c) => vec![*c],
            Slot::Charset(chars) => chars.clone(),
        })
        .collect();

    if options.is_empty() {
        return Ok(Vec::new());
    }

    // Odometer iteration
    let total = options.iter().map(|o| o.len()).product::<usize>();
    let mut results = Vec::with_capacity(total.min(1_000_000)); // pre-allocate reasonably
    let mut counters = vec![0usize; options.len()];

    for _ in 0..total {
        // Build current name from counters
        let name: String = counters
            .iter()
            .enumerate()
            .map(|(i, &c)| options[i][c])
            .collect();

        if is_valid_base_name(&name) {
            results.push(name);
        }

        // Increment odometer (rightmost first)
        let mut carry = true;
        for i in (0..counters.len()).rev() {
            if carry {
                counters[i] += 1;
                if counters[i] >= options[i].len() {
                    counters[i] = 0;
                } else {
                    carry = false;
                }
            }
        }
    }

    Ok(results)
}

/// Apply prefix and suffix permutations to a list of base names.
///
/// For each base name, generates combinations:
/// - prefix + name + suffix (for each prefix × suffix pair)
/// - prefix + name (for each prefix, if suffixes are empty or as standalone)
/// - name + suffix (for each suffix, if prefixes are empty or as standalone)
/// - name (bare, if `include_bare` is true)
///
/// All generated names are validated — invalid domain names are silently filtered.
pub fn apply_affixes<'a>(
    base_names: &'a [String],
    prefixes: &'a [String],
    suffixes: &'a [String],
    include_bare: bool,
) -> impl Iterator<Item = String> + 'a {
    base_names.iter().flat_map(move |name| {
        let mut variants = Vec::new();

        // prefix + name + suffix (all combinations)
        for prefix in prefixes {
            for suffix in suffixes {
                let candidate = format!("{}{}{}", prefix, name, suffix);
                if is_valid_base_name(&candidate) {
                    variants.push(candidate);
                }
            }
            // prefix + name (no suffix)
            if suffixes.is_empty() || !suffixes.is_empty() {
                let candidate = format!("{}{}", prefix, name);
                if is_valid_base_name(&candidate) {
                    variants.push(candidate);
                }
            }
        }

        // name + suffix (no prefix)
        for suffix in suffixes {
            let candidate = format!("{}{}", name, suffix);
            if is_valid_base_name(&candidate) {
                variants.push(candidate);
            }
        }

        // bare name
        if include_bare && is_valid_base_name(name) {
            variants.push(name.clone());
        }

        variants
    })
}

/// Run the full generation pipeline: patterns → affixes → validated names.
///
/// This is the main entry point for domain name generation. It:
/// 1. Expands all patterns into base names
/// 2. Combines with any literal base names (from `config.patterns` treated as patterns)
/// 3. Applies prefix/suffix permutations
/// 4. Filters all results through domain name validation
///
/// Returns a `GenerationResult` with the names and a pre-filter estimate.
pub fn generate_names(
    config: &GenerateConfig,
    literal_names: &[String],
) -> Result<GenerationResult, DomainCheckError> {
    // Step 1: Estimate total count (for informational purposes)
    let mut estimated_count: usize = literal_names.len();
    for pattern in &config.patterns {
        estimated_count = estimated_count.saturating_add(estimate_pattern_count(pattern)?);
    }

    // Step 2: Expand patterns into base names
    let mut base_names: Vec<String> = literal_names.to_vec();
    for pattern in &config.patterns {
        base_names.extend(expand_pattern(pattern)?);
    }

    // Step 3: Apply affixes if configured
    let names = if config.has_affixes() {
        apply_affixes(
            &base_names,
            &config.prefixes,
            &config.suffixes,
            config.include_bare,
        )
        .collect()
    } else {
        // Still filter through validation
        base_names
            .into_iter()
            .filter(|n| is_valid_base_name(n))
            .collect()
    };

    Ok(GenerationResult {
        names,
        estimated_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Pattern Parsing ─────────────────────────────────────────────

    #[test]
    fn test_literal_only() {
        let names = expand_pattern("test").unwrap();
        assert_eq!(names, vec!["test"]);
    }

    #[test]
    fn test_single_digit() {
        let names = expand_pattern("app\\d").unwrap();
        assert_eq!(names.len(), 10);
        assert!(names.contains(&"app0".to_string()));
        assert!(names.contains(&"app9".to_string()));
    }

    #[test]
    fn test_double_digit() {
        let names = expand_pattern("t\\d\\d").unwrap();
        assert_eq!(names.len(), 100);
        assert!(names.contains(&"t00".to_string()));
        assert!(names.contains(&"t99".to_string()));
        assert!(names.contains(&"t42".to_string()));
    }

    #[test]
    fn test_word_char() {
        let names = expand_pattern("x\\w").unwrap();
        // 26 letters + hyphen = 27, but "x-" ends with hyphen → filtered
        assert_eq!(names.len(), 26);
        assert!(names.contains(&"xa".to_string()));
        assert!(names.contains(&"xz".to_string()));
        assert!(!names.contains(&"x-".to_string())); // trailing hyphen filtered
    }

    #[test]
    fn test_word_char_leading_hyphen() {
        let names = expand_pattern("\\wa").unwrap();
        // 27 options, but "-a" starts with hyphen → filtered
        assert_eq!(names.len(), 26);
        assert!(names.contains(&"aa".to_string()));
        assert!(names.contains(&"za".to_string()));
        assert!(!names.contains(&"-a".to_string()));
    }

    #[test]
    fn test_question_mark() {
        let names = expand_pattern("x?").unwrap();
        // 37 chars (26 letters + hyphen + 10 digits), minus "x-" (trailing hyphen)
        assert_eq!(names.len(), 36);
        assert!(names.contains(&"xa".to_string()));
        assert!(names.contains(&"x0".to_string()));
        assert!(!names.contains(&"x-".to_string()));
    }

    #[test]
    fn test_mixed_pattern() {
        let names = expand_pattern("a\\d\\w").unwrap();
        // 10 digits × 27 word chars = 270 raw
        // Filter: names ending with hyphen (a0-, a1-, ... a9-) = 10 removed
        assert_eq!(names.len(), 260);
    }

    #[test]
    fn test_escaped_backslash() {
        let names = expand_pattern("test\\\\x");
        // Backslash is not valid in domain names, so it gets filtered
        assert!(names.is_ok());
        // "test\x" contains backslash, is_valid_base_name only allows alphanumeric + hyphen
        assert_eq!(names.unwrap().len(), 0);
    }

    #[test]
    fn test_empty_pattern_error() {
        let result = expand_pattern("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_escape_error() {
        let result = expand_pattern("test\\x");
        assert!(result.is_err());
        if let Err(DomainCheckError::InvalidPattern { reason, .. }) = result {
            assert!(reason.contains("\\x"));
        } else {
            panic!("Expected InvalidPattern error");
        }
    }

    #[test]
    fn test_trailing_backslash_error() {
        let result = expand_pattern("test\\");
        assert!(result.is_err());
    }

    #[test]
    fn test_only_wildcards_digits() {
        let names = expand_pattern("\\d\\d").unwrap();
        assert_eq!(names.len(), 100);
        assert!(names.contains(&"00".to_string()));
        assert!(names.contains(&"99".to_string()));
    }

    #[test]
    fn test_single_char_pattern_filtered() {
        // Single character patterns produce names < 2 chars → all filtered
        let names = expand_pattern("\\d").unwrap();
        assert_eq!(names.len(), 0); // all single-char, filtered by is_valid_base_name
    }

    // ── Estimates ───────────────────────────────────────────────────

    #[test]
    fn test_estimate_literal() {
        assert_eq!(estimate_pattern_count("test").unwrap(), 1);
    }

    #[test]
    fn test_estimate_digits() {
        assert_eq!(estimate_pattern_count("app\\d\\d").unwrap(), 100);
    }

    #[test]
    fn test_estimate_word() {
        assert_eq!(estimate_pattern_count("x\\w").unwrap(), 27);
    }

    #[test]
    fn test_estimate_question() {
        assert_eq!(estimate_pattern_count("x?").unwrap(), 37);
    }

    #[test]
    fn test_estimate_large() {
        // \w\w\w = 27^3 = 19,683
        assert_eq!(estimate_pattern_count("\\w\\w\\w").unwrap(), 19683);
    }

    // ── Affixes ─────────────────────────────────────────────────────

    #[test]
    fn test_prefix_only() {
        let base = vec!["app".to_string()];
        let result: Vec<_> =
            apply_affixes(&base, &["get".to_string(), "my".to_string()], &[], true).collect();
        assert!(result.contains(&"getapp".to_string()));
        assert!(result.contains(&"myapp".to_string()));
        assert!(result.contains(&"app".to_string())); // bare
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_suffix_only() {
        let base = vec!["app".to_string()];
        let result: Vec<_> =
            apply_affixes(&base, &[], &["ly".to_string(), "ify".to_string()], true).collect();
        assert!(result.contains(&"apply".to_string()));
        assert!(result.contains(&"appify".to_string()));
        assert!(result.contains(&"app".to_string())); // bare
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_prefix_and_suffix() {
        let base = vec!["cloud".to_string()];
        let prefixes = vec!["get".to_string()];
        let suffixes = vec!["ly".to_string()];
        let result: Vec<_> = apply_affixes(&base, &prefixes, &suffixes, true).collect();
        assert!(result.contains(&"getcloudly".to_string())); // prefix+name+suffix
        assert!(result.contains(&"getcloud".to_string())); // prefix+name
        assert!(result.contains(&"cloudly".to_string())); // name+suffix
        assert!(result.contains(&"cloud".to_string())); // bare
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_exclude_bare() {
        let base = vec!["app".to_string()];
        let result: Vec<_> = apply_affixes(&base, &["get".to_string()], &[], false).collect();
        assert!(result.contains(&"getapp".to_string()));
        assert!(!result.contains(&"app".to_string())); // bare excluded
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_empty_affixes_with_bare() {
        let base = vec!["test".to_string()];
        let result: Vec<_> = apply_affixes(&base, &[], &[], true).collect();
        assert_eq!(result, vec!["test".to_string()]);
    }

    #[test]
    fn test_affix_invalid_name_filtered() {
        // A prefix that produces a name starting with hyphen
        let base = vec!["app".to_string()];
        let result: Vec<_> = apply_affixes(&base, &["-".to_string()], &[], false).collect();
        // "-app" starts with hyphen → filtered
        assert!(result.is_empty());
    }

    // ── Pipeline ────────────────────────────────────────────────────

    #[test]
    fn test_pipeline_patterns_only() {
        let config = GenerateConfig {
            patterns: vec!["test\\d".to_string()],
            ..Default::default()
        };
        let result = generate_names(&config, &[]).unwrap();
        assert_eq!(result.names.len(), 10);
        assert!(result.names.contains(&"test0".to_string()));
    }

    #[test]
    fn test_pipeline_with_literals() {
        let config = GenerateConfig {
            patterns: vec!["app\\d".to_string()],
            ..Default::default()
        };
        let literals = vec!["mysite".to_string(), "example".to_string()];
        let result = generate_names(&config, &literals).unwrap();
        assert_eq!(result.names.len(), 12); // 2 literals + 10 from pattern
        assert!(result.names.contains(&"mysite".to_string()));
        assert!(result.names.contains(&"app5".to_string()));
    }

    #[test]
    fn test_pipeline_patterns_with_affixes() {
        let config = GenerateConfig {
            patterns: vec!["app\\d".to_string()],
            prefixes: vec!["get".to_string()],
            suffixes: vec![],
            include_bare: true,
        };
        let result = generate_names(&config, &[]).unwrap();
        // 10 patterns → each gets: getappN + appN = 20
        assert_eq!(result.names.len(), 20);
        assert!(result.names.contains(&"getapp0".to_string()));
        assert!(result.names.contains(&"app0".to_string()));
    }

    #[test]
    fn test_pipeline_literals_with_affixes() {
        let config = GenerateConfig {
            patterns: vec![],
            prefixes: vec!["my".to_string()],
            suffixes: vec!["hub".to_string()],
            include_bare: true,
        };
        let literals = vec!["cloud".to_string()];
        let result = generate_names(&config, &literals).unwrap();
        assert!(result.names.contains(&"mycloudhub".to_string()));
        assert!(result.names.contains(&"mycloud".to_string()));
        assert!(result.names.contains(&"cloudhub".to_string()));
        assert!(result.names.contains(&"cloud".to_string()));
        assert_eq!(result.names.len(), 4);
    }

    #[test]
    fn test_pipeline_empty_config() {
        let config = GenerateConfig::default();
        let result = generate_names(&config, &[]).unwrap();
        assert!(result.names.is_empty());
        assert_eq!(result.estimated_count, 0);
    }

    #[test]
    fn test_pipeline_estimated_count() {
        let config = GenerateConfig {
            patterns: vec!["x\\d\\d".to_string()],
            ..Default::default()
        };
        let result = generate_names(&config, &["literal".to_string()]).unwrap();
        assert_eq!(result.estimated_count, 101); // 1 literal + 100 pattern estimate
    }

    // ── GenerateConfig helpers ──────────────────────────────────────

    #[test]
    fn test_config_has_generation() {
        let empty = GenerateConfig::default();
        assert!(!empty.has_generation());

        let with_pattern = GenerateConfig {
            patterns: vec!["test".to_string()],
            ..Default::default()
        };
        assert!(with_pattern.has_generation());
    }

    #[test]
    fn test_config_has_affixes() {
        let empty = GenerateConfig::default();
        assert!(!empty.has_affixes());

        let with_prefix = GenerateConfig {
            prefixes: vec!["get".to_string()],
            ..Default::default()
        };
        assert!(with_prefix.has_affixes());
    }

    // ── Edge Cases ────────────────────────────────────────────────

    #[test]
    fn test_multiple_patterns_in_config() {
        let config = GenerateConfig {
            patterns: vec!["app\\d".to_string(), "go\\d".to_string()],
            ..Default::default()
        };
        let result = generate_names(&config, &[]).unwrap();
        assert_eq!(result.names.len(), 20); // 10 + 10
        assert!(result.names.contains(&"app0".to_string()));
        assert!(result.names.contains(&"go9".to_string()));
        assert_eq!(result.estimated_count, 20);
    }

    #[test]
    fn test_suffix_producing_trailing_hyphen() {
        let base = vec!["app".to_string()];
        let result: Vec<_> = apply_affixes(&base, &[], &["-".to_string()], false).collect();
        // "app-" ends with hyphen → filtered
        assert!(result.is_empty());
    }

    #[test]
    fn test_multiple_base_names_with_affixes() {
        let base = vec!["app".to_string(), "web".to_string()];
        let result: Vec<_> = apply_affixes(&base, &["my".to_string()], &[], true).collect();
        assert!(result.contains(&"myapp".to_string()));
        assert!(result.contains(&"myweb".to_string()));
        assert!(result.contains(&"app".to_string()));
        assert!(result.contains(&"web".to_string()));
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_long_literal_pattern() {
        let names = expand_pattern("superlongdomainname").unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "superlongdomainname");
    }

    #[test]
    fn test_single_question_mark_all_filtered() {
        // Single `?` produces 37 single-char names → all filtered (< 2 chars)
        let names = expand_pattern("?").unwrap();
        assert_eq!(names.len(), 0);
    }

    #[test]
    fn test_estimate_overflow_saturation() {
        // 5 wildcards: 37^5 = 69,343,957 — should not overflow
        let estimate = estimate_pattern_count("?????").unwrap();
        assert_eq!(estimate, 37usize.pow(5));
    }

    #[test]
    fn test_pattern_all_hyphens_filtered() {
        // Pattern "\\w\\w" where both are hyphens → "--" starts and ends with hyphen
        let names = expand_pattern("\\w\\w").unwrap();
        // "--" filtered (starts/ends with hyphen), but "a-", "-a" also filtered
        // Valid: only combos where neither first nor last is hyphen
        // 26 letters * 26 letters + 26 letters * 1 hyphen (middle allowed? no, only 2 chars)
        // Actually: first can be a-z (26), last can be a-z (26) → 676 valid
        // Plus first a-z, last hyphen → filtered. First hyphen, last a-z → filtered.
        assert_eq!(names.len(), 26 * 26); // 676 — only letter+letter combos
    }

    #[test]
    fn test_prefix_and_suffix_both_produce_invalid() {
        // prefix="-" suffix="-" → "-app-" starts and ends with hyphen
        let base = vec!["app".to_string()];
        let result: Vec<_> =
            apply_affixes(&base, &["-".to_string()], &["-".to_string()], false).collect();
        // "-app-" → invalid, "-app" → invalid, "app-" → invalid
        assert!(result.is_empty());
    }

    #[test]
    fn test_empty_base_names_with_affixes() {
        let base: Vec<String> = vec![];
        let result: Vec<_> = apply_affixes(&base, &["get".to_string()], &[], true).collect();
        assert!(result.is_empty());
    }

    #[test]
    fn test_pipeline_multiple_patterns_with_literals_and_affixes() {
        let config = GenerateConfig {
            patterns: vec!["x\\d".to_string()],
            prefixes: vec!["my".to_string()],
            suffixes: vec![],
            include_bare: true,
        };
        let literals = vec!["app".to_string()];
        let result = generate_names(&config, &literals).unwrap();
        // 1 literal + 10 pattern = 11 base names
        // Each gets: my{name} + {name} = 22
        assert_eq!(result.names.len(), 22);
        assert!(result.names.contains(&"myapp".to_string()));
        assert!(result.names.contains(&"app".to_string()));
        assert!(result.names.contains(&"myx0".to_string()));
        assert!(result.names.contains(&"x0".to_string()));
    }
}
