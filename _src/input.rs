use crate::util::block;

use crate::event::{
  Flow,
  start_listener,
};

use clipboard::{
  ClipboardContext,
  ClipboardProvider,
};

use crossterm::{
  queue,
  execute,

  cursor::{
    position,
    MoveToColumn,
  },

  event::{
    Event,
    KeyCode,
    KeyEvent,
    KeyModifiers,
    KeyEventKind,
  },

  style::{
    Color,
    Print,
    ResetColor,
    SetForegroundColor,
  },

  terminal::{
    size,
    Clear,
    ClearType,
    enable_raw_mode,
    disable_raw_mode,
    is_raw_mode_enabled,
  },
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

    start_listener(|event| {
      match event {
        // Enter
        Event::Key(KeyEvent {
          code: KeyCode::Enter,
          ..
        }) => {
          println!("");

          current_line = None;

          if !buffer.is_empty() {
            if let Some(old) = self.history.last() {
              if old == &buffer { return Flow::Break; }
            }

            if self.history.len() > 50 {
              self.history.remove(0);
            }

            self.history.push(buffer.clone());
            self.history_index = self.history.len();
          }

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

      // Print the candidate
      if candidate.len() > 0 {
        let mut candidate = candidate.join(", ");
        let w = size    ().unwrap().0;
        let x = position().unwrap().0;

        let rest = (w-x - 5) as usize;
        let show = candidate.len();

        // If rest is not sufficient to show all candidates
        if rest < show {
          // If rest is not sufficient to show any candidate
          if rest < 3 {
            candidate = String::new();
          }

          // If rest can show parts of the candidates
          else {
            candidate = candidate[..rest-1].to_string();
            candidate.push_str("..");
          }
        }

        // Add brackets
        if candidate.len() > 0 {
          candidate = format!("[{}]", candidate);
        }

        queue!(
          &stdout,
          SetForegroundColor(Color::DarkGrey),
          Print(" "),
          Print(candidate),
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
