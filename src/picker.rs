//! Picker state: filtering and selection only, no terminal I/O (see `tui`).

#[derive(Debug, Clone, Copy)]
pub enum Key {
    Up,
    Down,
    Char(char),
    Backspace,
    Enter,
    Cancel,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Pending,
    /// Index into the original items.
    Selected(usize),
    Cancelled,
}

pub struct Picker {
    items: Vec<String>,
    query: String,
    cursor: usize,
}

impl Picker {
    pub fn new(items: Vec<String>) -> Self {
        Picker {
            items,
            query: String::new(),
            cursor: 0,
        }
    }

    pub fn items(&self) -> &[String] {
        &self.items
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// Position of the highlight within `visible()`.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Indices of items matching the query (case-insensitive substring), in order.
    pub fn visible(&self) -> Vec<usize> {
        let q = self.query.to_lowercase();
        (0..self.items.len())
            .filter(|&i| self.items[i].to_lowercase().contains(&q))
            .collect()
    }

    /// `j`/`k`/`q` navigate/cancel only while the query is empty; once the user
    /// starts typing they're ordinary filter characters.
    pub fn handle(&mut self, key: Key) -> Outcome {
        let key = match key {
            Key::Char('j') if self.query.is_empty() => Key::Down,
            Key::Char('k') if self.query.is_empty() => Key::Up,
            Key::Char('q') if self.query.is_empty() => Key::Cancel,
            k => k,
        };
        let len = self.visible().len();
        match key {
            Key::Up => self.cursor = self.cursor.saturating_sub(1),
            Key::Down => self.cursor = (self.cursor + 1).min(len.saturating_sub(1)),
            Key::Char(c) => {
                self.query.push(c);
                self.cursor = 0;
            }
            Key::Backspace => {
                self.query.pop();
                self.cursor = 0;
            }
            Key::Enter => {
                if let Some(&i) = self.visible().get(self.cursor) {
                    return Outcome::Selected(i);
                }
            }
            Key::Cancel => return Outcome::Cancelled,
        }
        Outcome::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picker() -> Picker {
        Picker::new(vec!["alpha".into(), "Login-Fix".into(), "beta".into()])
    }

    #[test]
    fn enter_selects_the_highlighted_item_after_moving() {
        let mut p = picker();
        assert_eq!(p.handle(Key::Down), Outcome::Pending);
        assert_eq!(p.handle(Key::Enter), Outcome::Selected(1));
    }

    #[test]
    fn j_and_k_navigate_and_q_cancels_while_the_query_is_empty() {
        let mut p = picker();
        p.handle(Key::Char('j'));
        p.handle(Key::Char('j'));
        p.handle(Key::Char('k'));
        assert_eq!(p.handle(Key::Enter), Outcome::Selected(1));
        assert_eq!(picker().handle(Key::Char('q')), Outcome::Cancelled);
    }

    #[test]
    fn typing_filters_case_insensitively_and_selection_maps_to_original_index() {
        let mut p = picker();
        for c in "fix".chars() {
            p.handle(Key::Char(c));
        }
        assert_eq!(p.visible(), vec![1]);
        assert_eq!(p.handle(Key::Enter), Outcome::Selected(1));
    }

    #[test]
    fn once_filtering_letters_like_j_k_q_are_part_of_the_query() {
        let mut p = picker();
        p.handle(Key::Char('b'));
        p.handle(Key::Char('q'));
        assert_eq!(p.query(), "bq");
        assert!(p.visible().is_empty());
        assert_eq!(p.handle(Key::Enter), Outcome::Pending);
        p.handle(Key::Backspace);
        assert_eq!(p.handle(Key::Enter), Outcome::Selected(2));
    }

    #[test]
    fn cursor_stays_within_the_filtered_list() {
        let mut p = picker();
        for _ in 0..10 {
            p.handle(Key::Down);
        }
        assert_eq!(p.cursor(), 2);
        p.handle(Key::Char('a'));
        assert!(p.cursor() < p.visible().len());
    }

    #[test]
    fn escape_cancels() {
        assert_eq!(picker().handle(Key::Cancel), Outcome::Cancelled);
    }
}
