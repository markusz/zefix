//! German number formatting for the UI.

const THOUSANDS_GROUP: usize = 3;
pub const PERCENT: f64 = 100.0;

pub fn decimal(x: f64, digits: usize) -> String {
    format!("{x:.digits$}").replace('.', ",")
}

pub fn int(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / THOUSANDS_GROUP);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(THOUSANDS_GROUP) {
            out.push('.');
        }
        out.push(ch);
    }
    out
}

pub fn percent(p: f64) -> String {
    format!("{} %", decimal(PERCENT * p, 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn german_numbers() {
        assert_eq!(int(10_000_000), "10.000.000");
        assert_eq!(int(42), "42");
        assert_eq!(percent(0.16478), "16,48 %");
    }
}
