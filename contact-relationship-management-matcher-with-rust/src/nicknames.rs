//! English given-name nicknames.

use std::collections::HashMap;

use crate::normalize::fold;

/// Each row is one class: names that stand for the same person.
const CLASSES: &[&[&str]] = &[
    &["alexander", "alex", "sandy", "xander"],
    &["andrew", "andy", "drew"],
    &["anthony", "tony"],
    &["benjamin", "ben", "benny"],
    &[
        "catherine",
        "katherine",
        "kathryn",
        "kate",
        "katie",
        "kathy",
        "cathy",
        "kit",
    ],
    &["charles", "charlie", "chuck", "chas"],
    &["christopher", "chris", "kit"],
    &["daniel", "dan", "danny"],
    &["david", "dave", "davey"],
    &["edward", "ed", "eddie", "ted", "teddy", "ned"],
    &[
        "elizabeth",
        "liz",
        "beth",
        "betty",
        "eliza",
        "lizzie",
        "libby",
    ],
    &["jennifer", "jen", "jenny", "jenn"],
    &["john", "jon", "jack", "johnny", "jonny"],
    &["jonathan", "jon", "jonny", "nathan"],
    &["joseph", "joe", "joey"],
    &["james", "jim", "jimmy", "jamie"],
    &["margaret", "maggie", "meg", "peggy", "marge", "greta"],
    &["matthew", "matt", "matty"],
    &["michael", "mike", "mick", "mikey"],
    &["nicholas", "nick", "nicky"],
    &["patricia", "pat", "patty", "trish", "tricia"],
    &["richard", "rich", "rick", "ricky", "dick"],
    &["robert", "rob", "robbie", "bob", "bobby", "bert"],
    &["samuel", "sam", "sammy"],
    &["stephen", "steven", "steve"],
    &["thomas", "tom", "tommy"],
    &["william", "will", "bill", "billy", "liam"],
];

/// Which names stand for which. A name can belong to several classes
/// (`jon` is both a `john` and a `jonathan`).
#[derive(Clone, Debug)]
pub(crate) struct Nicknames {
    classes_of: HashMap<String, Vec<usize>>,
}

impl Nicknames {
    /// The built-in classes plus any extra ones from the configuration.
    pub(crate) fn new(extra: &[Vec<String>]) -> Self {
        let mut classes_of: HashMap<String, Vec<usize>> = HashMap::new();
        let built_in = CLASSES
            .iter()
            .map(|c| c.iter().map(|s| (*s).to_string()).collect());
        let all: Vec<Vec<String>> = built_in.chain(extra.iter().cloned()).collect();
        for (index, class) in all.iter().enumerate() {
            for name in class {
                let key = fold(name);
                if key.is_empty() {
                    continue;
                }
                let entry = classes_of.entry(key).or_default();
                if !entry.contains(&index) {
                    entry.push(index);
                }
            }
        }
        Self { classes_of }
    }

    /// Whether two folded names share a class.
    pub(crate) fn related(&self, a: &str, b: &str) -> bool {
        match (self.classes_of.get(a), self.classes_of.get(b)) {
            (Some(x), Some(y)) => x.iter().any(|c| y.contains(c)),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_classes() {
        let n = Nicknames::new(&[]);
        assert!(n.related("robert", "bob"));
        assert!(n.related("bob", "bobby"));
        assert!(n.related("john", "jon"));
        assert!(n.related("jon", "jonathan"));
        assert!(!n.related("john", "jonathan"));
        assert!(!n.related("robert", "william"));
        assert!(!n.related("zed", "zed2"));
    }

    #[test]
    fn extra_classes_extend_the_table() {
        let n = Nicknames::new(&[vec!["Zbigniew".into(), "Zbyszek".into()]]);
        assert!(n.related("zbigniew", "zbyszek"));
    }
}
