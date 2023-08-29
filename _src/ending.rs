

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
  None,
  CR,
  LF,
  CRLF,
}


impl std::str::FromStr for Ending {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "none" => Ok(Ending::None),
      "cr"   => Ok(Ending::CR  ),
      "lf"   => Ok(Ending::LF  ),
      "crlf" => Ok(Ending::CRLF),
      _      => Err(format!("Invalid ending: {}", s)),
    }
  }
}


impl std::fmt::Display for Ending {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Ending::None => write!(f, "None"),
      Ending::CR   => write!(f, "CR"  ),
      Ending::LF   => write!(f, "LF"  ),
      Ending::CRLF => write!(f, "CRLF"),
    }
  }
}
