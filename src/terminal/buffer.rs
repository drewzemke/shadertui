#[derive(Clone, PartialEq)]
pub struct Cell {
    pub content: String,
    pub is_empty: bool,
}

impl Cell {
    pub fn new() -> Self {
        Cell {
            content: " ".to_string(),
            is_empty: true,
        }
    }

    pub fn set_content(&mut self, content: String) {
        self.is_empty = content == " ";
        self.content = content;
    }
}

pub struct DoubleBuffer {
    pub current: Vec<Vec<Cell>>,
    pub next: Vec<Vec<Cell>>,
    pub width: usize,
    pub height: usize,
}

impl DoubleBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let current = vec![vec![Cell::new(); width]; height];
        let next = vec![vec![Cell::new(); width]; height];

        DoubleBuffer {
            current,
            next,
            width,
            height,
        }
    }

    pub fn set_cell(&mut self, x: usize, y: usize, content: String) {
        if x < self.width && y < self.height {
            self.next[y][x].set_content(content);
        }
    }

    pub fn clear_next(&mut self) {
        for row in &mut self.next {
            for cell in row {
                cell.set_content(" ".to_string());
            }
        }
    }

    pub fn swap_and_get_changes(&mut self) -> Vec<(usize, usize, String)> {
        let mut changes = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                if self.current[y][x] != self.next[y][x] {
                    changes.push((x, y, self.next[y][x].content.clone()));
                }
            }
        }

        std::mem::swap(&mut self.current, &mut self.next);
        changes
    }
}
