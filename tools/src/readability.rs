//! Reading level: a rough Flesch–Kincaid grade for prose, with math left
//! out. Used to keep passages near each test's level.

use crate::extra::prose_only;

fn words_of(s: &str) -> Vec<String> {
    prose_only(s)
        .split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
        .filter(|w| w.chars().any(char::is_alphabetic))
        .map(|w| w.to_lowercase())
        .collect()
}

/// Sentences of prose (math removed), split at . ! ? and blank lines.
pub fn sentences(s: &str) -> Vec<String> {
    prose_only(s)
        .split(['.', '!', '?'])
        .flat_map(|x| x.split("\n\n"))
        .map(str::trim)
        .filter(|x| x.chars().any(char::is_alphabetic))
        .map(str::to_string)
        .collect()
}

/// A rough syllable count: groups of vowels, minus a silent final "e".
pub fn syllables(word: &str) -> usize {
    let w: Vec<char> = word.to_lowercase().chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if w.is_empty() {
        return 0;
    }
    let vowel = |c: char| "aeiouy".contains(c);
    let mut count = 0;
    let mut prev = false;
    for &c in &w {
        let v = vowel(c);
        if v && !prev {
            count += 1;
        }
        prev = v;
    }
    if w.len() > 2 && w[w.len() - 1] == 'e' && !vowel(w[w.len() - 2]) && w[w.len() - 2] != 'l' {
        count -= 1;
    }
    count.max(1)
}

/// Flesch–Kincaid grade level of a piece of prose; 0 if it has no words.
pub fn grade(text: &str) -> f64 {
    let (mut n_sent, mut n_words, mut n_syl) = (0usize, 0usize, 0usize);
    for sent in sentences(text) {
        let w = words_of(&sent);
        if w.is_empty() {
            continue;
        }
        n_sent += 1;
        n_words += w.len();
        n_syl += w.iter().map(|x| syllables(x)).sum::<usize>();
    }
    if n_sent == 0 || n_words == 0 {
        return 0.0;
    }
    0.39 * (n_words as f64 / n_sent as f64) + 11.8 * (n_syl as f64 / n_words as f64) - 15.59
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllable_counts() {
        assert_eq!(syllables("cat"), 1);
        assert_eq!(syllables("factor"), 2);
        assert_eq!(syllables("table"), 2);
    }

    #[test]
    fn easy_text_reads_low_and_hard_text_reads_high() {
        assert!(grade("The cat sat. The dog ran. We had fun.") < 3.0);
        let hard = "Photosynthetic organisms convert electromagnetic radiation into chemical potential energy, \
                    subsequently utilized for biosynthesis of carbohydrates and maintenance of cellular homeostasis.";
        assert!(grade(hard) > 14.0);
    }
}
