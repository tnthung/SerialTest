use crossterm::{
  terminal::size,

  event::{
    read,
    Event,
  },
};


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Flow { Break, Continue }


pub fn start_listener<F>(mut cb: F, immediate: bool)
  where F: FnMut(Event) -> Flow
{
  if immediate {
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
}
