use crate::util::block;

use crate::event::{
  Flow,
  start_listener_sync,
};

use crossterm::{
  queue,
  execute,

  event::{
    Event,
    KeyCode,
    KeyEvent,
    KeyModifiers, KeyEventKind,
  },

  style::{
    Color,
    Print,
    ResetColor,
    SetForegroundColor,
  },

  terminal::{
    Clear,
    ClearType,
    enable_raw_mode,
    disable_raw_mode,
    is_raw_mode_enabled,
  }, cursor::MoveToColumn,
};


pub struct Processed {
  pub buffer   : Vec<String>,
  pub candidate: Vec<String>,
}


type Preprocessor<'a>    = Box<dyn FnMut(Vec<String>, usize) -> Processed + 'a>;
type Renderer<'a>        = Box<dyn FnMut(Vec<String>, usize) -> (String, usize) + 'a>;
type Finalizer<'a, T>    = Box<dyn FnMut(String) -> T + 'a>;
type FallbackHandler<'a> = Box<dyn FnMut(Event) -> Result<(), ()> + 'a>;


pub struct InputBuilder<'a, T> {
  prompt: String,

  preprocessor    : Option<Preprocessor   <'a>>,
  renderer        : Option<Renderer       <'a>>,
  finalizer       : Option<Finalizer      <'a, T>>,
  fallback_handler: Option<FallbackHandler<'a>>,
}


impl<'a, T> InputBuilder<'a, T> {
  pub fn new(prompt: impl AsRef<str>) -> Self {
    Self {
      prompt: prompt.as_ref().to_string(),

      preprocessor    : None,
      renderer        : None,
      finalizer       : None,
      fallback_handler: None,
    }
  }

  pub fn preprocessor(mut self, f: impl FnMut(Vec<String>, usize) -> Processed + 'a) -> Self {
    self.preprocessor = Some(Box::new(f));
    self
  }

  pub fn renderer(mut self, f: impl FnMut(Vec<String>, usize) -> (String, usize) + 'a) -> Self {
    self.renderer = Some(Box::new(f));
    self
  }

  pub fn fallback_handler(mut self, f: impl FnMut(Event) -> Result<(), ()> + 'a) -> Self {
    self.fallback_handler = Some(Box::new(f));
    self
  }

  pub fn build_with_final(self, f: impl FnMut(String) -> T + 'a) -> Input<'a, T> {
    Input {
      prompt: self.prompt,

      preprocessor    : self.preprocessor    .unwrap_or(Box::new(|s, _| Processed { buffer: s, candidate: Vec::new() })),
      renderer        : self.renderer        .unwrap_or(Box::new(|s, c| (s.join(""), c))),
      finalizer       : self.finalizer       .unwrap_or(Box::new(f)),
      fallback_handler: self.fallback_handler.unwrap_or(Box::new(|_| Ok(()))),

      history      : Vec::new(),
      history_index: 0,
    }
  }
}


impl<'a> InputBuilder<'a, String> {
  pub fn build(self) -> Input<'a, String> {
    self.build_with_final(|s| s)
  }
}


pub struct Input<'a, T> {
  prompt: String,

  preprocessor    : Preprocessor<'a>,
  renderer        : Renderer<'a>,
  finalizer       : Finalizer<'a, T>,
  fallback_handler: FallbackHandler<'a>,

  history      : Vec<Vec<String>>,
  history_index: usize,
}


impl<'a, T> Input<'a, T> {
  pub fn prompt(&mut self) -> Result<T, ()> {
    let enabled = is_raw_mode_enabled().unwrap();
    if !enabled { enable_raw_mode().unwrap(); }

    let stdout = std::io::stdout();

    let mut cursor    = 0usize;
    let mut buffer    = Vec::<String>::new();
    let mut candidate = Vec::<String>::new();

    let mut interrupted = false;

    execute!(
      &stdout,
      Print(&self.prompt)
    ).unwrap();

    start_listener_sync(|event| {
      match event {
        // Any key event that is not a press is omitted
        Event::Key(KeyEvent { kind, .. })
          if kind != KeyEventKind::Press => {},

        // Keyboard Interrupt
        Event::Key(KeyEvent {
          code     : KeyCode::Char('c'),
          modifiers: KeyModifiers::CONTROL,
          ..
        }) => {
          interrupted = true;
          return Flow::Break;
        },

        // Enter
        Event::Key(KeyEvent {
          code: KeyCode::Enter,
          ..
        }) => {
          println!("");

          if !buffer.is_empty() {
            self.history.push(buffer.clone());
            self.history_index = self.history.len();
          }

          return Flow::Break;
        },

        // Backspace
        Event::Key(KeyEvent {
          code: KeyCode::Backspace,
          ..
        }) => {
          if cursor > 0 {
            buffer.remove(cursor - 1);
            cursor -= 1;
          }
        },

        // Delete
        Event::Key(KeyEvent {
          code: KeyCode::Delete,
          ..
        }) => {
          if cursor < buffer.len() {
            buffer.remove(cursor);
          }
        },

        // Move cursor to the left
        Event::Key(KeyEvent {
          code: KeyCode::Left,
          modifiers,
          ..
        }) => {
          if cursor <= 0 { return Flow::Continue; }

          // Move cursor to the left by word
          if modifiers == KeyModifiers::CONTROL {
            let mut i = cursor - 1;
            while i > 0 && buffer[i - 1] != " " {
              i -= 1;
            }

            cursor = i;
          }

          // Move cursor to the left by character
          else {
            cursor -= 1;
          }
        },

        // Move cursor to the right
        Event::Key(KeyEvent {
          code: KeyCode::Right,
          modifiers,
          ..
        }) => block!({
          if cursor >= buffer.len() {
            // if the candidate is not empty, select the first candidate
            if !candidate.is_empty() {
              candidate[0].chars().for_each(|c| {
                buffer.insert(cursor, c.to_string());
                cursor += 1;
              });
              break;
            }

            return Flow::Continue;
          }

          // Move cursor to the right by word
          if modifiers == KeyModifiers::CONTROL {
            let mut i = cursor + 1;
            while i < buffer.len() && buffer[i] != " " {
              i += 1;
            }

            cursor = i;
          }

          // Move cursor to the right by character
          else {
            cursor += 1;
          }
        }),

        // Tab
        Event::Key(KeyEvent {
          code: KeyCode::Tab,
          ..
        }) => {
          if candidate.is_empty() { return Flow::Continue; }

          candidate[0].chars().for_each(|c| {
            buffer.insert(cursor, c.to_string());
            cursor += 1;
          });
        },

        // Previous history
        Event::Key(KeyEvent {
          code: KeyCode::Up,
          ..
        }) => {
          if self.history_index > 0 {
            self.history_index -= 1;

            let tmp = (self.preprocessor)(self.history[
              self.history_index].clone(), cursor);

            buffer    = tmp.buffer;
            cursor    = buffer.len();
            candidate = tmp.candidate;
          }
        },

        // Next history
        Event::Key(KeyEvent {
          code: KeyCode::Down,
          ..
        }) => {
          if self.history_index+1 < self.history.len() {
            self.history_index += 1;

            let tmp = (self.preprocessor)(self.history[
              self.history_index].clone(), cursor);

            buffer    = tmp.buffer;
            cursor    = buffer.len();
            candidate = tmp.candidate;
          }

          else {
            self.history_index = self.history.len();

            cursor    = 0;
            buffer    = Vec::new();
            candidate = Vec::new();
          }
        },

        // Move cursor to the start of the line
        Event::Key(KeyEvent {
          code: KeyCode::Home,
          ..
        }) => {
          cursor = 0;
        },

        // Move cursor to the end of the line
        Event::Key(KeyEvent {
          code: KeyCode::End,
          ..
        }) => {
          cursor = buffer.len();
        },

        // Insert character
        Event::Key(KeyEvent {
          code: KeyCode::Char(c),
          ..
        }) => {
          buffer.insert(cursor, c.to_string());
          cursor += 1;
        },

        // Anything else is handled through the fallback handler
        // If the fallback handler returns an error, the input
        // is interrupted
        _ => if let Err(_) = (self.fallback_handler)(event) {
          return Flow::Break;
        },
      }

      // calculate the old column
      let old_col = buffer[..cursor].iter()
        .fold(0, |acc, s| acc + s.len());

      // pre-process the buffer
      let tmp = (self.preprocessor)(
        buffer.clone(), cursor);

      buffer    = tmp.buffer;
      candidate = tmp.candidate;

      // calculate the new cursor
      cursor = {
        let mut tmp = 0usize;
        let mut col = old_col as i32;

        while col > 0 && tmp < buffer.len() {
          col -= buffer[tmp].len() as i32;
          tmp += 1;
        }

        tmp
      };

      // Render the buffer
      let (msg, col) = (self.renderer)(buffer.clone(), cursor);

      // Execute the render
      queue!(
        &stdout,
        Clear(ClearType::CurrentLine),
        MoveToColumn(0),
        Print(&self.prompt),
        Print(&msg),
      ).unwrap();

      if candidate.len() > 0 {
        queue!(
          &stdout,
          SetForegroundColor(Color::DarkGrey),
          Print(" ["),
          Print(&candidate.join(", ")),
          Print("]"),
        ).unwrap();
      }

      execute!(
        &stdout,
        ResetColor,
        MoveToColumn((col+self.prompt.len()) as u16),
      ).unwrap();

      Flow::Continue
    });


    if !enabled { disable_raw_mode().unwrap(); }

    // Return according to the interrupt status
    if interrupted { return Err(()); }
    Ok((self.finalizer)(buffer.join("")))
  }

  pub fn clear_history(&mut self) {
    self.history.clear();
    self.history_index = 0;
  }
}
