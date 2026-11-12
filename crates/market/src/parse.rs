//! Parse Polymarket daily-temperature bracket questions.
//!
//! Live shapes (Gamma `daily-temperature` tag, verified Nov 2026).
//! US markets use Fahrenheit ranges; intl markets use Celsius points:
//!
//! - "Will the highest temperature in New York City be 41F or below on November 13?"
//! - "Will the highest temperature in New York City be between 42-43F on November 13?"
//! - "Will the highest temperature in Atlanta be 64F or higher on November 13?"
//! - "Will the lowest temperature in London be -2C or below on November 13?"
//! - "Will the lowest temperature in London be 2C on November 13?"
//! - "Will the highest temperature in London be 11C or higher on November 13?"
//!
//! (Degree sign normalized away before parsing.)
//!
//! Brackets tile integer degrees: "41 or below" then "42-43" abut at 41.5.
//! The parser converts integer labels to continuous edges so model
//! probabilities tile to 1: below-X -> high = X+0.5, between A-B ->
//! low = A-0.5 / high = B+0.5, above-X -> low = X-0.5.
//! Celsius labels get the same half-degree treatment in C, then convert to F:
//! point X -> [(X-0.5)C, (X+0.5)C] in Fahrenheit.
//! All returned edges are Fahrenheit.

/// High (max temp) or Low (min temp) bracket.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BracketKind {
    High,
    Low,
}

/// A parsed bracket question with continuous degF edges.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedBracket {
    /// City name as written in the question ("New York City").
    pub city: String,
    pub kind: BracketKind,
    /// Continuous edges; None side = unbounded.
    pub low: Option<f64>,
    pub high: Option<f64>,
}

/// Parse a bracket question. Returns None when the question is not a
/// highest/lowest temperature bracket (caller logs + skips).
pub fn parse_bracket_question(question: &str) -> Option<ParsedBracket> {
    let normalized = question
        .replace('\u{00B0}', "")
        .replace(['\u{2013}', '\u{2014}', '\u{2212}'], "-");
    let lower = normalized.to_lowercase();

    let kind = if lower.contains("highest temperature") {
        BracketKind::High
    } else if lower.contains("lowest temperature") {
        BracketKind::Low
    } else {
        return None;
    };

    // City sits between "temperature in " and " be ".
    let city_marker = "temperature in ";
    let city_start = lower.find(city_marker)? + city_marker.len();
    let rest = &lower[city_start..];
    let be_pos = rest.find(" be ")?;
    let city = rest[..be_pos].trim();
    if city.is_empty() {
        return None;
    }

    let tail = &rest[be_pos + 4..]; // after " be "
                                    // Native-unit labels. Point brackets ("be 2C on ...") are (X, X);
                                    // the generic half-degree edge rule below turns them into [X-0.5, X+0.5].
    let below_marker = ["or below", "or lower"].iter().find(|m| tail.contains(*m));
    let above_marker = ["or above", "or higher"].iter().any(|m| tail.contains(m));
    let (low_label, high_label): (Option<f64>, Option<f64>) = if let Some(m) = below_marker {
        (None, Some(parse_single_degrees_before(tail, m)?))
    } else if above_marker {
        // Number sits right after "be ": "be 58f or above ..." / "be 11c or higher ...".
        (Some(number_at_start(tail)?), None)
    } else if tail.contains("between") {
        parse_between_degrees(tail)?
    } else {
        let x = number_at_start(tail)?;
        (Some(x), Some(x))
    };

    let unit = temp_unit(tail);
    // Integer labels -> continuous edges (half-degree in the native unit)...
    let (low_edge, high_edge) = match (low_label, high_label) {
        (None, None) => return None,
        (None, Some(h)) => (None, Some(h + 0.5)),
        (Some(l), None) => (Some(l - 0.5), None),
        (Some(l), Some(h)) => {
            if h < l {
                return None;
            }
            (Some(l - 0.5), Some(h + 0.5))
        }
    };
    // ...then to Fahrenheit.
    let to_f = |c: f64| c * 9.0 / 5.0 + 32.0;
    let (low, high) = match unit {
        Unit::F => (low_edge, high_edge),
        Unit::C => (low_edge.map(to_f), high_edge.map(to_f)),
    };

    Some(ParsedBracket {
        city: city.to_string(),
        kind,
        low,
        high,
    })
}

/// Temperature unit of the question's labels. Detected from the letter
/// directly following the first number ("41f ..." vs "2c ...") so month
/// names ("october") can't cause a misdetect.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Unit {
    F,
    C,
}

fn temp_unit(tail: &str) -> Unit {
    let mut prev_digit = false;
    let mut prev_dot = false;
    for ch in tail.chars() {
        if ch.is_ascii_digit() {
            prev_digit = true;
            prev_dot = false;
            continue;
        }
        if ch == '.' && prev_digit && !prev_dot {
            prev_dot = true;
            continue;
        }
        if prev_digit && (ch == 'f' || ch == 'c') {
            return if ch == 'f' { Unit::F } else { Unit::C };
        }
        prev_digit = false;
        prev_dot = false;
    }
    Unit::F
}

/// "41f or below ..." -> 41.0 : number immediately before the marker.
fn parse_single_degrees_before(tail: &str, marker: &str) -> Option<f64> {
    let pos = tail.find(marker)?;
    number_before(&tail[..pos])
}

/// "between 42-43f ..." -> (Some(42), Some(43)).
fn parse_between_degrees(tail: &str) -> Option<(Option<f64>, Option<f64>)> {
    let pos = tail.find("between")? + "between".len();
    let after = tail[pos..].trim_start();
    let dash = after.find('-')?;
    let left = number_before(&after[..dash])?;
    let right = number_at_start(&after[dash + 1..])?;
    Some((Some(left), Some(right)))
}

/// Trailing number in s ("...be 41f " -> 41). Tolerates trailing 'f'/'c'.
fn number_before(s: &str) -> Option<f64> {
    let s = s.trim_end();
    let s = s
        .strip_suffix('f')
        .or_else(|| s.strip_suffix('c'))
        .unwrap_or(s);
    let s = s.trim_end();
    let end = s.len();
    let mut start = end;
    let bytes = s.as_bytes();
    while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.') {
        start -= 1;
    }
    if start == end {
        return None;
    }
    // Include a leading minus if present.
    let start = if start > 0 && bytes[start - 1] == b'-' {
        start - 1
    } else {
        start
    };
    s[start..end].parse().ok()
}

/// Leading number in s ("43f on ..." -> 43).
fn number_at_start(s: &str) -> Option<f64> {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    let mut end = 0;
    if end < s.len() && bytes[end] == b'-' {
        end += 1;
    }
    while end < s.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
        end += 1;
    }
    if end == 0 {
        return None;
    }
    s[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_or_below() {
        let p = parse_bracket_question(
            "Will the highest temperature in New York City be 41\u{00B0}F or below on November 13?",
        )
        .unwrap();
        assert_eq!(p.city, "new york city");
        assert_eq!(p.kind, BracketKind::High);
        assert_eq!(p.low, None);
        assert_eq!(p.high, Some(41.5));
    }

    #[test]
    fn parses_between() {
        let p = parse_bracket_question(
            "Will the highest temperature in New York City be between 42-43\u{00B0}F on November 13?",
        )
        .unwrap();
        assert_eq!(p.low, Some(41.5));
        assert_eq!(p.high, Some(43.5));
    }

    #[test]
    fn parses_or_above() {
        let p = parse_bracket_question(
            "Will the highest temperature in New York City be 58\u{00B0}F or above on November 13?",
        )
        .unwrap();
        assert_eq!(p.low, Some(57.5));
        assert_eq!(p.high, None);
    }

    #[test]
    fn parses_or_higher_synonym() {
        let p = parse_bracket_question(
            "Will the highest temperature in Atlanta be 64\u{00B0}F or higher on November 13?",
        )
        .unwrap();
        assert_eq!(p.city, "atlanta");
        assert_eq!(p.low, Some(63.5));
        assert_eq!(p.high, None);
    }

    #[test]
    fn parses_celsius_point_to_fahrenheit() {
        // 2C point -> [1.5C, 2.5C] -> [34.7F, 36.5F].
        let p = parse_bracket_question(
            "Will the lowest temperature in London be 2\u{00B0}C on November 13?",
        )
        .unwrap();
        assert_eq!(p.city, "london");
        assert_eq!(p.kind, BracketKind::Low);
        let (low, high) = (p.low.unwrap(), p.high.unwrap());
        assert!((low - 34.7).abs() < 1e-9, "low={low}");
        assert!((high - 36.5).abs() < 1e-9, "high={high}");
    }

    #[test]
    fn parses_celsius_below_negative() {
        // -2C or below -> high = -1.5C = 29.3F.
        let p = parse_bracket_question(
            "Will the lowest temperature in London be -2\u{00B0}C or below on November 13?",
        )
        .unwrap();
        assert_eq!(p.low, None);
        let high = p.high.unwrap();
        assert!((high - 29.3).abs() < 1e-9, "high={high}");
    }

    #[test]
    fn parses_celsius_or_higher() {
        // 11C or higher -> low = 10.5C = 50.9F.
        let p = parse_bracket_question(
            "Will the highest temperature in London be 11\u{00B0}C or higher on November 13?",
        )
        .unwrap();
        assert_eq!(p.kind, BracketKind::High);
        let low = p.low.unwrap();
        assert!((low - 50.9).abs() < 1e-9, "low={low}");
        assert_eq!(p.high, None);
    }

    #[test]
    fn celsius_edges_tile_without_gaps() {
        let below = parse_bracket_question(
            "Will the lowest temperature in London be -2C or below on November 13?",
        )
        .unwrap();
        let point =
            parse_bracket_question("Will the lowest temperature in London be -1C on November 13?")
                .unwrap();
        assert!((below.high.unwrap() - point.low.unwrap()).abs() < 1e-9);
    }

    #[test]
    fn parses_lowest_between() {
        let p = parse_bracket_question(
            "Will the lowest temperature in Chicago be between 28-29\u{00B0}F on November 12?",
        )
        .unwrap();
        assert_eq!(p.city, "chicago");
        assert_eq!(p.kind, BracketKind::Low);
        assert_eq!(p.low, Some(27.5));
        assert_eq!(p.high, Some(29.5));
    }

    #[test]
    fn parses_lowest_or_below() {
        let p = parse_bracket_question(
            "Will the lowest temperature in Chicago be 27\u{00B0}F or below on November 12?",
        )
        .unwrap();
        assert_eq!(p.kind, BracketKind::Low);
        assert_eq!(p.low, None);
        assert_eq!(p.high, Some(27.5));
    }

    #[test]
    fn rejects_non_temperature() {
        assert!(parse_bracket_question("Will it rain in New York City on November 13?").is_none());
        assert!(parse_bracket_question("Xi Jinping out before 2027?").is_none());
        assert!(parse_bracket_question("").is_none());
    }

    #[test]
    fn edges_tile_without_gaps() {
        // Adjacent brackets must share exact edges.
        let below = parse_bracket_question(
            "Will the highest temperature in Miami be 70F or below on Nov 1?",
        )
        .unwrap();
        let between = parse_bracket_question(
            "Will the highest temperature in Miami be between 71-72F on Nov 1?",
        )
        .unwrap();
        assert_eq!(below.high, between.low);
    }
}
