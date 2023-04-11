//! DRQL "scanner"
//!
//! This module provides the scanner -- or the tool that searches a string of text
//! for DRQL queries enclosed in `@{ ... }` and returns an Iterator over their
//! contents.

use lazy_static::lazy_static;
use regex::Regex;

/// Returns an Iterator over provided text, returning every value within `@{ ... }`.
pub fn scan(input: &str) -> impl Iterator<Item = &'_ str> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"@\{(.+?)\}").unwrap();
    }
    RE.find_iter(input)
        .map(|m| &m.as_str()[2..(m.as_str().len() - 1)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_returns_none_with_empty_input() {
        assert_eq!(scan("").next(), None);
    }

    #[test]
    fn scan_is_not_greedy() {
        assert_eq!(scan("a@{b}c@{d}e").collect::<Vec<_>>(), vec!["b", "d"]);
    }

    #[test]
    fn scan_larger() {
        assert_eq!(
            scan("Hello @{everyone - here}! Come online please! @{staff} as well.")
                .collect::<Vec<_>>(),
            vec!["everyone - here", "staff"]
        );
    }
}
