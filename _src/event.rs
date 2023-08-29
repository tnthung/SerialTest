use crossterm::{
  event::{
    read,
    Event,
  },

  terminal::{
    size,
    enable_raw_mode,
    disable_raw_mode,
    is_raw_mode_enabled,
  },
};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flow {
  Break,
  Continue,
}


pub fn start_listener<F>(mut cb: F)
  where F: FnMut(Event) -> Flow
{
  let enabled = is_raw_mode_enabled().unwrap();
  if !enabled { enable_raw_mode().unwrap(); }

  {
    let (c, r) = size().unwrap();
    cb(Event::Resize(c, r));
  }

  loop {
    let event = read().unwrap();

    match cb(event) {
      Flow::Break    => break,
      Flow::Continue => continue,
    }
  }

  if !enabled { disable_raw_mode().unwrap(); }
}
