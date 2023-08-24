

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
  None,           // invalid command
  Help,           // print help
  Clear,          // clear screen

  Flush,          // flush serial port
  Send,           // send data
  Receive,        // receive data

  SetMode,        // set mode
  SetEnding,      // set ending

  SetPort,        // set port
  SetBaud,        // set baud rate
  SetDataBits,    // set data bits
  SetParity,      // set parity
  SetStopBits,    // set stop bits
  SetTimeout,     // set timeout
  SetFlow,        // set flow control

  SetRts,         // set RTS
  SetDtr,         // set DTR

  GetMode,        // get mode
  GetEnding,      // get ending

  GetPort,        // get port
  GetBaud,        // get baud rate
  GetDataBits,    // get data bits
  GetParity,      // get parity
  GetStopBits,    // get stop bits
  GetTimeout,     // get timeout
  GetFlow,        // get flow control

  GetInQue,       // get input queue
  GetOutQue,      // get output queue

  GetCts,         // get CTS
  GetDsr,         // get DSR
  GetRi,          // get RI
  GetCd,          // get CD
}

impl std::str::FromStr for CommandType {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "help"       => Ok(CommandType::Help       ),
      "clear"      => Ok(CommandType::Clear      ),
      "flush"      => Ok(CommandType::Flush      ),
      "send"       => Ok(CommandType::Send       ),
      "recv"       => Ok(CommandType::Receive    ),
      "set-mode"   => Ok(CommandType::SetMode    ),
      "set-ending" => Ok(CommandType::SetEnding  ),
      "set-port"   => Ok(CommandType::SetPort    ),
      "set-baud"   => Ok(CommandType::SetBaud    ),
      "set-data"   => Ok(CommandType::SetDataBits),
      "set-par"    => Ok(CommandType::SetParity  ),
      "set-stop"   => Ok(CommandType::SetStopBits),
      "set-time"   => Ok(CommandType::SetTimeout ),
      "set-flow"   => Ok(CommandType::SetFlow    ),
      "set-rts"    => Ok(CommandType::SetRts     ),
      "set-dtr"    => Ok(CommandType::SetDtr     ),
      "get-mode"   => Ok(CommandType::GetMode    ),
      "get-ending" => Ok(CommandType::GetEnding),
      "get-port"   => Ok(CommandType::GetPort    ),
      "get-baud"   => Ok(CommandType::GetBaud    ),
      "get-data"   => Ok(CommandType::GetDataBits),
      "get-par"    => Ok(CommandType::GetParity  ),
      "get-stop"   => Ok(CommandType::GetStopBits),
      "get-time"   => Ok(CommandType::GetTimeout ),
      "get-flow"   => Ok(CommandType::GetFlow    ),
      "get-in"     => Ok(CommandType::GetInQue   ),
      "get-out"    => Ok(CommandType::GetOutQue  ),
      "get-cts"    => Ok(CommandType::GetCts     ),
      "get-dsr"    => Ok(CommandType::GetDsr     ),
      "get-ri"     => Ok(CommandType::GetRi      ),
      "get-cd"     => Ok(CommandType::GetCd      ),
      _            => Ok(CommandType::None       ),
    }
  }
}

impl std::fmt::Display for CommandType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CommandType::None        => write!(f, "None"       ),
      CommandType::Help        => write!(f, "Help"       ),
      CommandType::Clear       => write!(f, "Clear"      ),
      CommandType::Flush       => write!(f, "Flush"      ),
      CommandType::Send        => write!(f, "Send"       ),
      CommandType::Receive     => write!(f, "Receive"    ),
      CommandType::SetMode     => write!(f, "SetMode"    ),
      CommandType::SetEnding   => write!(f, "SetEnding"  ),
      CommandType::SetPort     => write!(f, "SetPort"    ),
      CommandType::SetBaud     => write!(f, "SetBaud"    ),
      CommandType::SetDataBits => write!(f, "SetDataBits"),
      CommandType::SetParity   => write!(f, "SetParity"  ),
      CommandType::SetStopBits => write!(f, "SetStopBits"),
      CommandType::SetTimeout  => write!(f, "SetTimeout" ),
      CommandType::SetFlow     => write!(f, "SetFlow"    ),
      CommandType::SetRts      => write!(f, "SetRts"     ),
      CommandType::SetDtr      => write!(f, "SetDtr"     ),
      CommandType::GetMode     => write!(f, "GetMode"    ),
      CommandType::GetEnding   => write!(f, "GetEnding"  ),
      CommandType::GetPort     => write!(f, "GetPort"    ),
      CommandType::GetBaud     => write!(f, "GetBaud"    ),
      CommandType::GetDataBits => write!(f, "GetDataBits"),
      CommandType::GetParity   => write!(f, "GetParity"  ),
      CommandType::GetStopBits => write!(f, "GetStopBits"),
      CommandType::GetTimeout  => write!(f, "GetTimeout" ),
      CommandType::GetFlow     => write!(f, "GetFlow"    ),
      CommandType::GetInQue    => write!(f, "GetInQue"   ),
      CommandType::GetOutQue   => write!(f, "GetOutQue"  ),
      CommandType::GetCts      => write!(f, "GetCts"     ),
      CommandType::GetDsr      => write!(f, "GetDsr"     ),
      CommandType::GetRi       => write!(f, "GetRi"      ),
      CommandType::GetCd       => write!(f, "GetCd"      ),
    }
  }
}
