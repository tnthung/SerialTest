use std::ops::{AddAssign, SubAssign, Sub};

use crate::util::block;

use crate::listener::{
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


#[derive(Default)]
pub struct Environment {
  pub prompt   : String,
  pub buffer   : Vec<String>,
  pub cursor   : usize,
  pub column   : usize,
  pub rendered : String,
  pub candidate: Vec<String>,
  pub flow_ctrl: Option<Flow>,
  pub interrupt: bool,
  pub history  : Vec<Vec<String>>,
  pub history_i: usize,
}


type Preprocessor   <'a   > = Box<dyn FnMut(&mut Environment       ) -> () + 'a>;
type Renderer       <'a   > = Box<dyn FnMut(&mut Environment       ) -> () + 'a>;
type Finalizer      <'a, T> = Box<dyn FnMut(&mut Environment       ) -> T  + 'a>;
type FallbackHandler<'a   > = Box<dyn FnMut(&mut Environment, Event) -> () + 'a>;


pub struct InputBuilder<'a, T> {
  prompt          : String,
  preprocessor    : Option<Preprocessor   <'a   >>,
  renderer        : Option<Renderer       <'a   >>,
  finalizer       : Option<Finalizer      <'a, T>>,
  fallback_handler: Option<FallbackHandler<'a   >>,
}


impl<'a, T> InputBuilder<'a, T> {
  pub fn new() -> Self {
    Self {
      prompt          : String::new(),
      preprocessor    : None,
      renderer        : None,
      finalizer       : None,
      fallback_handler: None,
    }
  }

  pub fn prompt(mut self, prompt: String) -> Self {
    self.prompt = prompt;
    self
  }

  pub fn preprocessor(mut self, preprocessor: Preprocessor<'a>) -> Self {
    self.preprocessor = Some(preprocessor);
    self
  }

  pub fn renderer(mut self, renderer: Renderer<'a>) -> Self {
    self.renderer = Some(renderer);
    self
  }

  pub fn fallback_handler(mut self, fallback_handler: FallbackHandler<'a>) -> Self {
    self.fallback_handler = Some(fallback_handler);
    self
  }

  pub fn build_with_final(self, finalizer: Finalizer<'a, T>) -> Input<'a, T> {
    Input {
      env: Default::default(),

      preprocessor    : self.preprocessor    .unwrap_or_else(|| Box::new(|_|    {})),
      renderer        : self.renderer        .unwrap_or_else(|| Box::new(|_|    {})),
      fallback_handler: self.fallback_handler.unwrap_or_else(|| Box::new(|_, _| {})),

      finalizer,
    }
  }
}

impl<'a> InputBuilder<'a, String> {
  pub fn build(self) -> Input<'a, String> {
    self.build_with_final(Box::new(|env| env.buffer.join("")))
  }
}


pub struct Input<'a, T> {
  env: Environment,

  preprocessor    : Preprocessor   <'a   >,
  renderer        : Renderer       <'a   >,
  finalizer       : Finalizer      <'a, T>,
  fallback_handler: FallbackHandler<'a   >,
}


impl<'a, T> Input<'a, T> {
  pub fn prompt(&mut self) {
    enable_raw_mode().unwrap();

    let stdout = std::io::stdout();

    start_listener(|event| {
      match event {
        // Omit non-press events
        Event::Key(KeyEvent { kind, .. }) if kind != KeyEventKind::Press => {},

        Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. }) => self.interrupt(),
        Event::Key(KeyEvent { code: KeyCode::Char('v'), modifiers: KeyModifiers::CONTROL, .. }) => self.paste(),

        Event::Key(KeyEvent { code: KeyCode::Char(c), .. }) => self.insert(c),

        Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) => self.backspace(),
        Event::Key(KeyEvent { code: KeyCode::Delete   , .. }) => self.delete(),

        Event::Key(KeyEvent { code: KeyCode::Left , modifiers: KeyModifiers::CONTROL, .. }) => self.move_left_word(),
        Event::Key(KeyEvent { code: KeyCode::Left ,                                   .. }) => self.move_left(),

        Event::Key(KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::CONTROL, .. }) => self.move_right_word(),
        Event::Key(KeyEvent { code: KeyCode::Right,                                   .. }) => self.move_right(),

        Event::Key(KeyEvent { code: KeyCode::Home, .. }) => self.move_to_start(),
        Event::Key(KeyEvent { code: KeyCode::End , .. }) => self.move_to_end(),

        Event::Key(KeyEvent { code: KeyCode::Tab, .. }) => self.autocomplete(),

        Event::Key(KeyEvent { code: KeyCode::Up  , .. }) => self.previous_history(),
        Event::Key(KeyEvent { code: KeyCode::Down, .. }) => self.next_history(),

        _ => (self.fallback_handler)(&mut self.env, event),
      }

      if let Some(flow) = self.env.flow_ctrl {
        self.env.flow_ctrl = None;
        return flow;
      }

      let swap = self.env.history_i == self.env.history.len();


      Flow::Continue
    }, true);
  }

  fn clear_history(&mut self) {
    todo!()
  }

  fn previous_history(&mut self) {
    let cursor    = &mut self.env.cursor;
    let history   = &mut self.env.history;
    let history_i = &mut self.env.history_i;

    if *history_i <= 0 {
      self.env.flow_ctrl = Some(Flow::Continue);
      return;
    }

    *history_i -= 1;
    *cursor     = history[*history_i].len();
  }

  fn next_history(&mut self) {
    let buffer    = &mut self.env.buffer;
    let cursor    = &mut self.env.cursor;
    let history   = &mut self.env.history;
    let history_i = &mut self.env.history_i;

    if *history_i >= history.len() {
      self.env.flow_ctrl = Some(Flow::Continue);
      return;
    }

    *history_i += 1;
    *cursor =
      if *history_i == history.len() { buffer.len() }
      else { history[*history_i].len() };
  }

  fn paste(&mut self) {
    let buffer = &mut self.env.buffer;
    let cursor = &mut self.env.cursor;

    if let Ok(s) = ClipboardContext::new()
      .unwrap()
      .get_contents()
    {
      let before = buffer[..*cursor].to_vec();
      let after  = buffer[*cursor..].to_vec();

      *buffer = before;
      buffer.extend(s.chars()
        .into_iter()
        .map(|c| c.to_string()));
      buffer.extend(after);

      *cursor += s.len();
    }
  }

  fn backspace(&mut self) {
    let buffer = &mut self.env.buffer;
    let cursor = &mut self.env.cursor;

    if *cursor > 0 {
      *cursor -= 1;
      buffer.remove(*cursor);
    }
  }

  fn delete(&mut self) {
    let buffer = &mut self.env.buffer;
    let cursor = &mut self.env.cursor;

    if *cursor < buffer.len() {
      buffer.remove(*cursor);
    }
  }

  fn move_left_word(&mut self) {
    let buffer = &mut self.env.buffer;
    let cursor = &mut self.env.cursor;

    if *cursor == 0 {
      self.env.flow_ctrl = Some(Flow::Continue);
      return;
    }

    let mut i = *cursor;

    while i > 0 && buffer[i-1] != " " { i -= 1; }

    *cursor = i;
  }

  fn move_right_word(&mut self) {
    let buffer = &mut self.env.buffer;
    let cursor = &mut self.env.cursor;

    if *cursor == buffer.len() {
      self.env.flow_ctrl = Some(Flow::Continue);
      return;
    }

    let mut i = *cursor;

    while i < buffer.len() && buffer[i] != " " { i += 1; }

    *cursor = i;
  }

  fn move_left(&mut self) {
    let cursor = &mut self.env.cursor;

    if *cursor == 0 {
      self.env.flow_ctrl = Some(Flow::Continue);
      return;
    }

    *cursor -= 1;
  }

  fn move_right(&mut self) {
    let buffer = &mut self.env.buffer;
    let cursor = &mut self.env.cursor;

    if *cursor == buffer.len() {
      self.env.flow_ctrl = Some(Flow::Continue);
      return;
    }

    *cursor += 1;
  }

  fn move_to_start(&mut self) {
    self.env.cursor = 0;
  }

  fn move_to_end(&mut self) {
    let buffer = &mut self.env.buffer;
    let cursor = &mut self.env.cursor;

    *cursor = buffer.len();
  }

  fn autocomplete(&mut self) {
    let buffer    = &mut self.env.buffer;
    let cursor    = &mut self.env.cursor;
    let candidate = &mut self.env.candidate;

    if candidate.is_empty() {
      self.env.flow_ctrl = Some(Flow::Continue);
      return;
    }

    candidate[0].chars().for_each(|c| {
      buffer.insert(*cursor, c.to_string());
      *cursor += 1;
    });
  }

  fn accept(&mut self) {
    todo!()
  }

  fn interrupt(&mut self) {
    self.env.flow_ctrl = Some(Flow::Break);
    self.env.interrupt = true;
  }

  fn insert(&mut self, c: char) {
    let buffer    = &mut self.env.buffer;
    let cursor    = &mut self.env.cursor;
    let history   = &mut self.env.history;
    let history_i = &mut self.env.history_i;

    if *history_i != history.len() {
      *buffer = history[*history_i].clone();
      *history_i = history.len();
    }

    buffer.insert(*cursor, c.to_string());
    *cursor += 1;
  }
}
