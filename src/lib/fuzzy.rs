use std::cmp;

#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    pub text: String,
    pub score: f64,
}

pub struct FuzzySearcher {
    jaro_threshold: f64,
}

impl FuzzySearcher {
    pub fn new(jaro_threshold: f64) -> Self {
        Self { jaro_threshold }
    }

    pub fn search(&self, query: &str, candidates: &[String]) -> Vec<FuzzyMatch> {
        let mut matches: Vec<FuzzyMatch> = candidates
            .iter()
            .filter_map(|candidate| {
                if let Some(score) = self.fuzzy_match(query, candidate) {
                    Some(FuzzyMatch {
                        text: candidate.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by score (higher is better)
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        self.decide_match(&matches)
    }

    fn decide_match(&self, ordered_matches: &Vec<FuzzyMatch>) -> Vec<FuzzyMatch> {
        if ordered_matches.len() <= 1 {
            return ordered_matches.clone();
        }

        // If the first match has a score of 1.0, just return it
        if let Some(first_match) = ordered_matches.first() {
            if first_match.score >= 0.99 {
                return vec![first_match.clone()];
            }
        }

        // if the first match has a score > 0.9 and it's 20% more than the second match,
        // we can consider it a strong match
        if let Some(first_match) = ordered_matches.first() {
            if let Some(second_match) = ordered_matches.get(1) {
                if first_match.score > 0.9 && first_match.score >= second_match.score * 1.2 {
                    return vec![first_match.clone()];
                }
            }
        }

        ordered_matches.clone()
    }

    fn fuzzy_match(&self, query: &str, candidate: &str) -> Option<f64> {
        let query_lower = query.to_lowercase();
        let candidate_lower = candidate.to_lowercase();

        // First check: exact match
        if query_lower == candidate_lower {
            return Some(1.0); // Exact match gets the highest score
        }

        // Second check: subsequence matching (characters in order)
        if self.is_subsequence(&query_lower, &candidate_lower) {
            let jaro_score = self.jaro_similarity(&query_lower, &candidate_lower);
            // Direct subsequence match gets a high base score
            return Some(jaro_score);
        }

        // Third check: Jaro similarity for "close" matches
        let jaro_score = self.jaro_similarity(&query_lower, &candidate_lower);
        if jaro_score >= self.jaro_threshold {
            Some(jaro_score)
        } else {
            None
        }
    }

    fn is_subsequence(&self, query: &str, candidate: &str) -> bool {
        let mut query_chars = query.chars().peekable();
        let mut candidate_chars = candidate.chars();

        while let Some(query_char) = query_chars.peek() {
            let mut found = false;
            for candidate_char in candidate_chars.by_ref() {
                if *query_char == candidate_char {
                    query_chars.next();
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }

        query_chars.peek().is_none()
    }

    fn jaro_similarity(&self, s1: &str, s2: &str) -> f64 {
        if s1 == s2 {
            return 1.0;
        }

        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        let len1 = s1_chars.len();
        let len2 = s2_chars.len();

        if len1 == 0 || len2 == 0 {
            return 0.0;
        }

        let match_window = cmp::max(len1, len2) / 2;
        let match_window = if match_window > 0 {
            match_window - 1
        } else {
            0
        };

        let mut s1_matches = vec![false; len1];
        let mut s2_matches = vec![false; len2];

        let mut matches = 0;

        // Find matches
        for i in 0..len1 {
            let start = if i >= match_window {
                i - match_window
            } else {
                0
            };
            let end = cmp::min(i + match_window + 1, len2);

            for j in start..end {
                if s2_matches[j] || s1_chars[i] != s2_chars[j] {
                    continue;
                }
                s1_matches[i] = true;
                s2_matches[j] = true;
                matches += 1;
                break;
            }
        }

        if matches == 0 {
            return 0.0;
        }

        // Count transpositions
        let mut transpositions = 0;
        let mut k = 0;

        for i in 0..len1 {
            if !s1_matches[i] {
                continue;
            }
            while !s2_matches[k] {
                k += 1;
            }
            if s1_chars[i] != s2_chars[k] {
                transpositions += 1;
            }
            k += 1;
        }

        let jaro = (matches as f64 / len1 as f64
            + matches as f64 / len2 as f64
            + (matches - transpositions / 2) as f64 / matches as f64)
            / 3.0;

        jaro
    }
}

// Convenience function for simple usage
pub fn fuzzy_search(query: &str, candidates: &[String]) -> Vec<String> {
    let searcher = FuzzySearcher::new(0.6); // Default threshold
    searcher
        .search(query, candidates)
        .into_iter()
        .map(|m| m.text)
        .collect()
}

// Example usage and tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subsequence_matching() {
        let searcher = FuzzySearcher::new(0.6);

        // Test basic subsequence matching
        assert!(searcher.is_subsequence("abc", "aabbcc"));
        assert!(searcher.is_subsequence("abc", "axbxcx"));
        assert!(searcher.is_subsequence("abc", "abcdef"));
        assert!(!searcher.is_subsequence("abc", "acb"));
        assert!(!searcher.is_subsequence("abc", "ab"));
    }

    #[test]
    fn test_fuzzy_search() {
        let candidates = vec![
            "hello world".to_string(),
            "helloworld".to_string(),
            "help".to_string(),
            "world".to_string(),
            "rust programming".to_string(),
            "programming".to_string(),
            "program".to_string(),
            "progress".to_string(),
        ];

        let results = fuzzy_search("prog", &candidates);

        // Should match "program", "programming", "progress" due to subsequence
        assert!(results.contains(&"program".to_string()));
        assert!(results.contains(&"programming".to_string()));
        assert!(results.contains(&"progress".to_string()));
    }

    #[test]
    fn test_with_scores() {
        let searcher = FuzzySearcher::new(0.5);
        let candidates = vec![
            "test".to_string(),
            "best".to_string(),
            "rest".to_string(),
            "testing".to_string(),
        ];

        let results = searcher.search("test", &candidates);

        // "test" should have the highest score (exact match)
        assert!(!results.is_empty());
        assert_eq!(results[0].text, "test");
        assert!(results[0].score > 0.9);
    }
}
