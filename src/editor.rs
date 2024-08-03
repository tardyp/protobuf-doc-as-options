#[derive(Debug, Clone)]
pub struct Insertion {
    position: usize,
    text: String,
}
#[derive(Debug, Clone, Copy)]
pub struct Deletion {
    position: usize,
    length: usize,
}
#[derive(Debug, Clone)]
pub enum Edition {
    Insert(Insertion),
    Delete(Deletion),
}
impl Edition {
    pub fn position(&self) -> usize {
        match self {
            Edition::Insert(i) => i.position,
            Edition::Delete(d) => d.position,
        }
    }
}

pub struct Editor {
    text: String,
    line_offsets: Vec<usize>,
    editions: Vec<Edition>,
}
fn gen_offsets(text: &str) -> Vec<usize> {
    let mut line_offsets = vec![0];
    let mut i = 0;
    for c in text.chars() {
        i += c.len_utf8();
        if c == '\n' {
            line_offsets.push(i);
        }
    }
    line_offsets
}
impl Editor{
    pub fn new(text: String) -> Self {
        Self {
            line_offsets: gen_offsets(&text),
            text,
            editions: Vec::new(),
        }
    }
    pub fn get_position(&self, line: usize, column: usize) -> usize {
        let line = line.min(self.line_offsets.len() - 1);
        let line = self.line_offsets[line];
        let column = column.min(self.text().len() - line);
        line + column
    }
    pub fn insert(&mut self, position: usize, text: String) {
        self.editions.push(Edition::Insert(Insertion {
            position,
            text,
        }));
    }
    pub fn delete(&mut self, position: usize, length: usize) {
        self.editions.push(Edition::Delete(Deletion {
            position,
            length,
        }));
    }
    pub fn apply(&mut self) {
        let mut new_text = String::new();
        let mut editions = self.editions.clone();
        editions.sort_by_key(|d| (d.position(), match d {
            Edition::Insert(_) => 0,
            Edition::Delete(_) => 1,
        }));
        let mut last_position = 0;
        for edition in editions {
            if last_position < edition.position() {
                new_text.push_str(&self.text[last_position..edition.position()]);
            }
            match edition {
                Edition::Insert(i) => {
                    new_text.push_str(&i.text);
                    last_position = i.position;
                }
                Edition::Delete(d) => {
                    last_position = d.position + d.length;
                }
            }
        }
        new_text.push_str(&self.text[last_position..]);
        self.line_offsets = gen_offsets(&new_text);
        self.text = new_text;
        self.editions.clear();
    }
    
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_editor() {
        let mut editor = Editor::new("Hello, world!".to_string());
        editor.insert(7, "beautiful ".to_string());
        editor.delete(0, 7);
        editor.apply();
        assert_eq!(editor.text, "beautiful world!");
    }
    #[test]
    fn test_complex_editor() {
        let mut editor = Editor::new("Hello, world!".to_string());
        editor.insert(7, "beautiful".to_string());
        editor.delete(0, 7);
        editor.insert(0, "Goodbye ".to_string());
        editor.delete(7, 6);
        editor.apply();
        assert_eq!(editor.text, "Goodbye beautiful");
    }
    #[test]
    fn test_gen_offset_emoji() {
        // for correct slice indexing, we need to use byte offsets
        // see https://doc.rust-lang.org/std/primitive.str.html#method.get_unchecked
        let editor = Editor::new("👋\n🌍\ntest".to_string());
        assert_eq!(editor.get_position(0, 0), 0);
        assert_eq!(editor.get_position(0, 1), 1);
        assert_eq!(editor.get_position(1, 0), 5);
        assert_eq!(editor.get_position(2, 4), editor.text().len());
        assert_eq!("test", &editor.text()[editor.get_position(2, 0)..]);
    }
}