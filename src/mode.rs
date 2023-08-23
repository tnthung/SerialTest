

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
  ASCII,
  HEX,
}


impl std::str::FromStr for Mode {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "ascii" => Ok(Mode::ASCII),
      "hex"   => Ok(Mode::HEX  ),
      _       => Err(()),
    }
  }
}


impl std::fmt::Display for Mode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Mode::ASCII => write!(f, "ASCII"),
      Mode::HEX   => write!(f, "HEX"),
    }
  }
}
