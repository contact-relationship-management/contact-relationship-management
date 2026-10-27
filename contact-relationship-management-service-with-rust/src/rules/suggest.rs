//! Suggesting help articles for a ticket (CRM-R66): which published articles share words with its
//! title. Plain word overlap in integers, so the order is the same everywhere and can be explained.

use std::collections::BTreeSet;

/// Most suggestions given.
pub const MAX_SUGGESTIONS: usize = 5;
/// A word counts once in the title or keywords of an article…
const HEADLINE_WEIGHT: u32 = 2;
/// …and once, less, in its text.
const BODY_WEIGHT: u32 = 1;

/// Words too common to say anything.
const STOP: [&str; 24] = [
    "the", "and", "for", "are", "but", "not", "you", "can", "with", "has", "have", "this", "that",
    "from", "was", "our", "its", "when", "what", "how", "why", "does", "did", "into",
];

/// The words of `text`: lower case, letters and digits, at least three characters, not common.
#[must_use]
pub fn words(text: &str) -> BTreeSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3 && !STOP.contains(w))
        .map(str::to_string)
        .collect()
}

/// An article offered for a ticket.
#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    /// Index into the articles given.
    pub index: usize,
    /// Headline words count 2, text words 1.
    pub score: u32,
    /// The words of the ticket the article shares, in alphabetical order.
    pub matched: Vec<String>,
}

/// The articles `(title, keywords, body)` that share a word with `ticket_title`, best first (more
/// score, then the earlier article), at most [`MAX_SUGGESTIONS`]. A ticket with no usable words
/// gets none.
#[must_use]
pub fn suggest(ticket_title: &str, articles: &[(String, Option<String>, String)]) -> Vec<Match> {
    let wanted = words(ticket_title);
    let mut found: Vec<Match> = articles
        .iter()
        .enumerate()
        .filter_map(|(index, (title, keywords, body))| {
            let headline = words(&format!("{title} {}", keywords.as_deref().unwrap_or("")));
            let text = words(body);
            let (mut score, mut matched) = (0, Vec::new());
            for word in &wanted {
                if headline.contains(word) {
                    score += HEADLINE_WEIGHT;
                } else if text.contains(word) {
                    score += BODY_WEIGHT;
                } else {
                    continue;
                }
                matched.push(word.clone());
            }
            (score > 0).then_some(Match {
                index,
                score,
                matched,
            })
        })
        .collect();
    found.sort_by(|a, b| b.score.cmp(&a.score).then(a.index.cmp(&b.index)));
    found.truncate(MAX_SUGGESTIONS);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn article(
        title: &str,
        keywords: Option<&str>,
        body: &str,
    ) -> (String, Option<String>, String) {
        (title.into(), keywords.map(Into::into), body.into())
    }

    /// Short and common words are dropped; case and punctuation do not matter.
    #[test]
    fn words_are_lower_case_and_meaningful() {
        let w = words("The Licence-seats are NOT showing, in 2FA!");
        assert_eq!(
            w.into_iter().collect::<Vec<_>>(),
            vec!["2fa", "licence", "seats", "showing"]
        );
    }

    /// Headline words outweigh text words, ties go to the earlier article, and nothing shared
    /// means nothing offered.
    #[test]
    fn the_best_overlap_comes_first() {
        let articles = vec![
            article("Billing", None, "licence seats are counted monthly"),
            article("Licence seats", Some("console"), "Open the console."),
            article("Passwords", None, "reset it"),
            article("Seats", None, "licence"),
        ];
        let got = suggest("Licence seats not showing in console", &articles);
        let order: Vec<usize> = got.iter().map(|m| m.index).collect();
        assert_eq!(order, vec![1, 3, 0]);
        assert_eq!(got[0].score, 6);
        assert_eq!(got[0].matched, vec!["console", "licence", "seats"]);
        assert_eq!(got[1].score, 3);
        assert!(suggest("!!", &articles).is_empty());
        assert!(suggest("Quarterly forecast", &articles).is_empty());
    }

    /// No more than five are given.
    #[test]
    fn at_most_five() {
        let articles: Vec<_> = (0..9).map(|_| article("Sensor", None, "")).collect();
        assert_eq!(suggest("sensor", &articles).len(), MAX_SUGGESTIONS);
    }
}
