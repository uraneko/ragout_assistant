use std::io::{StdoutLock, Write};

use crossterm::terminal::enable_raw_mode;

// raw mode:
// you need to create exetrns for C functions from unistd.h
// Specifically to enable raw mode you need tcgetattr and tcsetattr functions.

///
/// Enables terminal raw mode and initializes the necessary variables for behaving in the raw mode.
///
/// Takes a [`&str`] for the shell prompt (give "" for no prompt) and a bool for the option of running in the terminal alternate screen (give true to run your cli program in alternate screen)
/// # Errors
/// Does NOT return errors and never panics
///
/// # Example
///
/// Basic usage
///
/// ```
/// use ragout::{init, run};
///
/// fn main() {
///     // enter raw mode and initialize necessary variables
///     // the string literal argument will be the value of the prompt
///     let (mut sol, mut i, mut h, mut ui) = init("some prompt 🐱 ", true);
///
///     'main: loop {
///         let input = run(&mut i, &mut h, &mut sol, &mut ui);
///         if !input.is_empty() {
///             // do some stuff with the user input
///         }
///     }
/// }
/// ```
pub fn init(
    prompt: &str,
    alt_screen: bool,
) -> (std::io::StdoutLock<'static>, Input, History, String) {
    _ = enable_raw_mode();

    let mut sol = std::io::stdout().lock();

    if alt_screen {
        _ = sol.write(b"\x1b[?1049h");
        _ = sol.write(b"\x1b[1;1f");
    }

    let i = Input::new(prompt, alt_screen);
    i.write_prompt(&mut sol);

    (sol, i, History::new(), String::new())
}

/// A struct that implements the user input movement and deletion logic inside the terminal raw
/// mode
#[derive(Debug)]
pub struct Input {
    pub values: Vec<char>,
    pub cursor: usize,
    #[cfg(any(debug_assertions, feature = "debug_logs"))]
    pub debug_log: std::fs::File,
    pub prompt: String,
    pub alt_screen: bool,
}

impl Input {
    /// Creates a new Input instance
    pub fn new(prompt: &str, alt_screen: bool) -> Self {
        Self {
            #[cfg(any(debug_assertions, feature = "debug_logs"))]
            debug_log: std::fs::File::create("resources/logs/terminal/input").unwrap_or_else(
                |_| {
                    std::fs::create_dir_all("resources/logs/terminal").unwrap();
                    std::fs::File::create("resources/logs/terminal/input").unwrap()
                },
            ),
            values: Vec::new(),
            cursor: 0,
            prompt: prompt.to_owned(),
            alt_screen,
        }
    }

    // NOTE: should input.values not be a byte vec instead of a char vec?
    /// Adds inputted char to Input values at cursor position then increments Input cursor
    pub fn put_char(&mut self, c: char) {
        match self.values.is_empty() {
            true => {
                self.values.push(c);
                self.cursor += 1;
            }
            false => match self.cursor == self.values.len() {
                true => {
                    self.values.push(c);
                    self.cursor += 1;
                }

                false => {
                    self.values.insert(self.cursor, c);
                    self.cursor += 1;
                }
            },
        }
    }

    // TODO: multiline input
    // WARN: do NOT touch this Input implementation
    // the fns other than write are not to be touched

    /// Pushs Input values to history, then binds a [`String`] of the Input values to user_input and resets both Input cursor and values
    pub fn cr_lf(&mut self, h: &mut History, user_input: &mut String) {
        h.push(self.values.to_vec());
        *user_input = self.values.drain(..).collect::<String>();
        self.cursor = 0;
    }

    /// Deletes the char behind the cursor position in the Input values
    pub fn backspace(&mut self) {
        if self.values.is_empty() || self.cursor == 0 {
            return;
        }
        if self.cursor > 0 {
            self.values.remove(self.cursor - 1);
            self.cursor -= 1;
        }
    }

    /// Moves the Input cursor one cell to the right
    pub fn to_the_right(&mut self) -> bool {
        if self.values.is_empty() || self.cursor == self.values.len() {
            return false;
        }
        self.cursor += 1;

        true
    }

    /// Moves the Input cursor one cell to the left
    pub fn to_the_left(&mut self) -> bool {
        if self.values.is_empty() || self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;

        true
    }

    /// Moves Input cursor to the position after the last in Input values (which is values.len())
    pub fn to_end(&mut self) -> usize {
        let diff = self.values.len() - self.cursor;
        if diff > 0 {
            self.cursor = self.values.len();
        }

        diff
    }

    /// Moves Input cursor to the first position in Input values (which is 0)
    pub fn to_home(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor = 0;

        true
    }

    /// Clears all the Input values
    pub fn clear_line(&mut self) {
        self.cursor = 0;
        self.values.clear();
    }

    /// clears the values of Input to the right of Input cursor
    pub fn clear_right(&mut self) {
        for _ in self.cursor..self.values.len() {
            self.values.pop();
        }
    }

    /// clears the values of Input to the left of Input cursor
    pub fn clear_left(&mut self) {
        for _ in 0..self.cursor {
            self.values.remove(0);
        }
        self.cursor = 0;
    }

    const STOPPERS: [char; 11] = ['/', ' ', '-', '_', ',', '"', '\'', ';', ':', '.', ','];

    /// Syncs Input's internal state to a movement of the user input cursor to the right, stops at the first stopper char
    pub fn to_right_jump(&mut self) {
        if self.cursor == self.values.len() {
            return;
        }

        match self.values[if self.cursor + 1 < self.values.len() {
            self.cursor + 1
        } else {
            self.cursor
        }] == ' '
        {
            true => {
                while self.cursor + 1 < self.values.len() && self.values[self.cursor + 1] == ' ' {
                    self.cursor += 1;
                }
            }
            false => {
                while self.cursor + 1 < self.values.len()
                    && !Self::STOPPERS.contains(&self.values[self.cursor + 1])
                {
                    self.cursor += 1;
                }
                self.cursor += 1;
            }
        }
    }

    /// Syncs Input's internal state to a movement of the user input cursor to the left, stops at the first stopper char
    pub fn to_left_jump(&mut self) {
        if self.cursor == 0 {
            return;
        }

        match self.values[self.cursor - 1] == ' ' {
            true => {
                while self.cursor > 0 && self.values[self.cursor - 1] == ' ' {
                    self.cursor -= 1;
                }
            }
            false => {
                while self.cursor > 1 && !Self::STOPPERS.contains(&self.values[self.cursor - 1]) {
                    self.cursor -= 1;
                }
                self.cursor -= 1;
            }
        }
    }
}

// NOTE: the cursor in both input and history does not point to the item it's on,
// but is alawys pointing at the item to the left
// basically cursor = 0 points at nothing and cursor = 4 points at eg. input[3]
// this logic is implemented in the functionality

#[derive(Debug)]
pub struct History {
    #[cfg(any(debug_assertions, feature = "debug_logs"))]
    pub debug_log: std::fs::File,
    pub values: Vec<Vec<char>>,
    pub cursor: usize,
    pub temp: Option<Vec<char>>,
}

impl History {
    /// Creates a new History instance
    pub fn new() -> Self {
        Self {
            #[cfg(any(debug_assertions, feature = "debug_logs"))]
            debug_log: std::fs::File::create("resources/logs/terminal/history").unwrap_or_else(
                |_| {
                    std::fs::create_dir_all("resources/logs/terminal").unwrap();
                    std::fs::File::create("resources/logs/terminal/history").unwrap()
                },
            ),
            values: Vec::new(),
            cursor: 0,
            temp: None,
        }
    }

    /// Binds the value of the previous history entry to the value variable and moves back the
    /// History cursor by one
    pub fn prev(&mut self, value: &mut Vec<char>) -> bool {
        if self.cursor == 0 {
            return false;
        }

        if self.temp.is_none() || self.cursor == self.values.len() {
            self.temp = Some(value.clone()); // temporarily keep input val
        }

        *value = self.values[self.cursor - 1].clone();
        self.cursor -= 1;

        true
    }

    /// Binds the value of the next history entry to the value variable and moves forward the
    /// History cursor by one
    pub fn next(&mut self, value: &mut Vec<char>) -> bool {
        if self.cursor == self.values.len() {
            return false;
        }

        if self.cursor + 1 == self.values.len() {
            *value = self.temp.as_ref().unwrap().clone();
        } else {
            *value = self.values[self.cursor + 1].clone();
        }
        self.cursor += 1;

        true
    }

    /// Pushs a new history entry into the History.values
    pub fn push(&mut self, value: Vec<char>) {
        if value.iter().filter(|c| **c != ' ').count() > 0 && !self.values.contains(&value) {
            self.values.push(value);
        }
        self.temp = None;
        self.cursor = self.values.len();
    }
}

#[cfg(test)]
mod test_input {
    use super::{History, Input};

    #[test]
    fn test_put_char() {
        let mut i = Input::new("testing input> ", false);

        let mut idx = 0;
        ['p', 'i', 'k', 'a'].into_iter().for_each(|c| {
            i.put_char(c);
            idx += 1;

            assert_eq!(i.values[i.cursor - 1], c);
            assert_eq!(idx, i.cursor);
        })
    }

    #[test]
    fn test_backspace() {
        let mut i = Input::new("testing input> ", false);

        let input = "pikatchino";
        input.chars().into_iter().for_each(|c| i.put_char(c));

        i.backspace();

        assert!({ i.cursor == input.len() - 1 && i.values[i.cursor - 1] == 'n' });
    }

    #[test]
    fn test_to_end() {
        let mut i = Input::new("testing input> ", false);

        "pikatchaa".chars().into_iter().for_each(|c| i.put_char(c));
        // cursor is by default at end, but we still move it to end
        i.to_end();

        assert!({ i.cursor == 9 && i.values[i.cursor - 1] == 'a' });

        // now we test moving to end from somewhere else
        i.to_the_left();
        i.to_the_left();
        i.to_end();

        assert!({ i.cursor == 9 && i.values[i.cursor - 1] == 'a' });

        // and finally, moving to end from home (first cell in line)
        i.to_home();
        i.to_end();

        assert!({ i.cursor == 9 && i.values[i.cursor - 1] == 'a' });
    }

    #[test]
    fn test_to_home() {
        let mut i = Input::new("testing input> ", false);

        "pikatchuu".chars().into_iter().for_each(|c| i.put_char(c));
        i.to_home();

        assert!({ i.cursor == 0 && i.values[i.cursor] == 'p' });
    }

    #[test]
    fn test_to_the_right() {
        let mut i = Input::new("testing input> ", false);

        "pikatchau".chars().into_iter().for_each(|c| i.put_char(c));
        i.to_the_left();
        i.to_the_left();

        assert_eq!(i.values[i.cursor - 1], 'h');
        assert_eq!(i.cursor, "pikatchau".len() - 2);
    }

    #[test]
    fn test_to_the_left() {
        let mut i = Input::new("testing input> ", false);

        "pikatchau".chars().into_iter().for_each(|c| i.put_char(c));
        i.to_home();
        i.to_the_right();
        i.to_the_right();

        assert_eq!(i.values[i.cursor], 'k');
        assert_eq!(i.cursor, 2);
    }

    #[test]
    fn test_cr_lf() {
        let mut i = Input::new("testing input> ", false);
        let mut h = History::new();
        let mut user_input = String::new();

        "pikatcharu".chars().into_iter().for_each(|c| i.put_char(c));

        i.cr_lf(&mut h, &mut user_input);

        assert_eq!(
            h.values[0],
            "pikatcharu".chars().into_iter().collect::<Vec<char>>()
        );
        assert!(i.values.is_empty());
        assert_eq!(i.cursor, 0);
    }

    #[test]
    fn test_clear_line() {
        let mut i = Input::new("testing input> ", false);

        "pikauchi".chars().into_iter().for_each(|c| i.put_char(c));

        assert!({ i.cursor == "pikauchi".len() && i.values[i.cursor - 1] == 'i' });

        i.clear_line();
        assert!(i.values.is_empty());
        assert_eq!(i.cursor, 0);
    }

    #[test]
    fn test_clear_right() {
        let mut i = Input::new("testing input> ", false);

        "pikatchiatto"
            .chars()
            .into_iter()
            .for_each(|c| i.put_char(c));
        (0..4).for_each(|_| {
            i.to_the_left();
        });

        i.clear_right();
        assert_eq!(i.values.iter().map(|c| *c).collect::<String>(), "pikatchi");
    }

    #[test]
    fn test_clear_left() {
        let mut i = Input::new("testing input> ", false);

        "pikatchiatto"
            .chars()
            .into_iter()
            .for_each(|c| i.put_char(c));
        (0..4).for_each(|_| {
            i.to_the_left();
        });

        i.clear_left();
        assert_eq!(i.values.iter().map(|c| *c).collect::<String>(), "atto");
    }
}

impl Input {
    /// Changes the Input prompt value to the provided string
    pub fn overwrite_prompt(&mut self, new_prompt: &str) {
        self.prompt.clear();
        self.prompt.push_str(new_prompt);
    }

    /// Renders the Input prompt followed by the Input values on a clean line
    pub fn write_prompt(&self, sol: &mut StdoutLock) {
        _ = sol.write(b"\x1b[2K");
        _ = sol.write(&[13]);
        _ = sol.write(&str_to_bytes(&self.prompt));
        _ = sol.write(&str_to_bytes(&self.as_str(&mut "".to_string())));
        _ = sol.flush();
    }

    /// Syncs the user input cursor displayed in the terminal to the cursor of Input
    pub fn sync_cursor(&self, sol: &mut StdoutLock) {
        _ = sol.write(&[13]);
        // BUG: at every first inputted char of an input line, the cursor was moving forward
        // by the sum of the byte lengths of all non-ascii chars in the prompt
        // this is because prompt(String).len() was counting the byte lengths of the chars not the
        // number of the chars
        // FIX: switch to prompt.chars.count() from prompt.len()
        for _idx in 0..self.prompt.chars().count() + 1 + self.cursor {
            _ = sol.write(b"\x1b[C");
        }
    }

    // pub fn toggle_alt_screen(&mut self, sol: &mut StdoutLock) {
    //     match self.alt_screen {
    //         true => {
    //             _ = sol.write(b"\x1b[?1049l");
    //         }
    //         false => {
    //             _ = sol.write(b"\x1b[?1049h");
    //         }
    //     }
    //
    //     self.alt_screen = !self.alt_screen;
    // }
    fn as_str<'a>(&self, s: &'a mut String) -> &'a str {
        *s = self.values.iter().map(|c| c).collect::<String>();

        s.as_str()
    }
}

fn encode_char(c: char, bytes: &mut Vec<u8>) {
    match c.is_ascii() {
        false => bytes.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes()),
        true => bytes.push(c as u8),
    }
}

fn str_to_bytes(s: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    s.chars()
        .into_iter()
        .for_each(|c| encode_char(c, &mut bytes));

    bytes
}
