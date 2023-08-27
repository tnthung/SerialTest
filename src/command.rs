

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandType {
  None,           // invalid command
  Help,           // print help
  Clear,          // clear screen

  Flush,          // flush serial port
  Send,           // send data
  Receive,        // receive data

  SetMode,        // set mode
  SetEnding,      // set ending
  SetReverse,     // set reverse

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
  GetReverse,     // get reverse

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
      "set-end"    => Ok(CommandType::SetEnding  ),
      "set-rev"    => Ok(CommandType::SetReverse ),
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
      "get-end"    => Ok(CommandType::GetEnding  ),
      "get-rev"    => Ok(CommandType::GetReverse ),
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
      CommandType::Help        => write!(f, "help"      ),
      CommandType::Clear       => write!(f, "clear"     ),
      CommandType::Flush       => write!(f, "flush"     ),
      CommandType::Send        => write!(f, "send"      ),
      CommandType::Receive     => write!(f, "recv"      ),
      CommandType::SetMode     => write!(f, "set-mode"  ),
      CommandType::SetEnding   => write!(f, "set-end"   ),
      CommandType::SetReverse  => write!(f, "set-rev"   ),
      CommandType::SetPort     => write!(f, "set-port"  ),
      CommandType::SetBaud     => write!(f, "set-baud"  ),
      CommandType::SetDataBits => write!(f, "set-data"  ),
      CommandType::SetParity   => write!(f, "set-par"   ),
      CommandType::SetStopBits => write!(f, "set-stop"  ),
      CommandType::SetTimeout  => write!(f, "set-time"  ),
      CommandType::SetFlow     => write!(f, "set-flow"  ),
      CommandType::SetRts      => write!(f, "set-rts"   ),
      CommandType::SetDtr      => write!(f, "set-dtr"   ),
      CommandType::GetMode     => write!(f, "get-mode"  ),
      CommandType::GetEnding   => write!(f, "get-end"   ),
      CommandType::GetReverse  => write!(f, "get-rev"   ),
      CommandType::GetPort     => write!(f, "get-port"  ),
      CommandType::GetBaud     => write!(f, "get-baud"  ),
      CommandType::GetDataBits => write!(f, "get-data"  ),
      CommandType::GetParity   => write!(f, "get-par"   ),
      CommandType::GetStopBits => write!(f, "get-stop"  ),
      CommandType::GetTimeout  => write!(f, "get-time"  ),
      CommandType::GetFlow     => write!(f, "get-flow"  ),
      CommandType::GetInQue    => write!(f, "get-in"    ),
      CommandType::GetOutQue   => write!(f, "get-out"   ),
      CommandType::GetCts      => write!(f, "get-cts"   ),
      CommandType::GetDsr      => write!(f, "get-dsr"   ),
      CommandType::GetRi       => write!(f, "get-ri"    ),
      CommandType::GetCd       => write!(f, "get-cd"    ),
      CommandType::None        => write!(f, "none"      ),
    }
  }
}


impl CommandType {
  pub fn get_help(&self) -> &str {
    match *self {
      CommandType::Help  =>
"help [command]: Print the help information for the command. \
If no command is specified, print the help information for all commands.

  - [command]: The command to print help information for.",

      CommandType::Send =>
"send <message>: Send the message to the serial port and immediately receive the response.

  - <message>: The message to send.",

      CommandType::SetMode =>
"set-mode <mode>: Set the mode of the message.

  - <mode>: The mode of the message.
    ascii: ASCII mode.
    hex  : Hexadecimal mode.",

      CommandType::SetEnding =>
"set-end: <ending>: Set the ending of the message.

  - <ending>: The type of ending.
    none: No ending.
    cr  : \\r.
    lf  : \\n.
    crlf: \\r\\n.",

      CommandType::SetReverse =>
"set-rev <reverse>: Set if message is in reverse order.

  - <state>: Whether the message is in reverse order.
    on : Reverse order.
    off: Normal order.",

      CommandType::SetPort =>
"set-port <port>: Switch to the specified serial port.

  - <port>: The serial port to switch to.",

      CommandType::SetBaud =>
"set-baud <baud>: Set the baud rate of the serial port.

  - <baud>: The baud rate of the serial port.",

      CommandType::SetDataBits =>
"set-data <data>: Set the data bits of the serial port.

  - <data>: The data bits of the serial port.
    5: 5 bits.
    6: 6 bits.
    7: 7 bits.
    8: 8 bits.",

      CommandType::SetParity =>
"set-par <parity>: Set the parity of the serial port.

  - <parity>: The parity of the serial port.
    none: No parity.
    odd : Odd parity.
    even: Even parity.",

      CommandType::SetStopBits =>
"set-stop <stop>: Set the stop bits of the serial port.

  - <stop>: The stop bits of the serial port.
    1: 1 bit.
    2: 2 bits.",

      CommandType::SetTimeout =>
"set-time <timeout>: Set the timeout of the serial port.

  - <timeout>: The timeout of the serial port.",

      CommandType::SetFlow =>
"set-flow <flow>: Set the flow control of the serial port.

  - <flow>: The flow control of the serial port.
    none    : No flow control.
    hardware: Hardware flow control.
    software: Software flow control.",

      CommandType::SetRts =>
"set-rts <state>: Set the RTS of the serial port.

  - <state>: Whether the RTS is on.
    on : RTS on.
    off: RTS off.",

      CommandType::SetDtr =>
"set-dtr <state>: Set the DTR of the serial port.

  - <state>: Whether the DTR is on.
    on : DTR on.
    off: DTR off.",

      CommandType::Clear       => "clear: Clear the screen.",
      CommandType::Flush       => "flush: Flush the serial port manually.",
      CommandType::Receive     => "recv: Receive the message from the serial port.",
      CommandType::GetMode     => "get-mode: Get the mode of the message.",
      CommandType::GetEnding   => "get-end: Get the ending of the message.",
      CommandType::GetReverse  => "get-rev: Get if message is in reverse order.",
      CommandType::GetPort     => "get-port: Get the serial port.",
      CommandType::GetBaud     => "get-baud: Get the baud rate of the serial port.",
      CommandType::GetDataBits => "get-data: Get the data bits of the serial port.",
      CommandType::GetParity   => "get-par: Get the parity of the serial port.",
      CommandType::GetStopBits => "get-stop: Get the stop bits of the serial port.",
      CommandType::GetTimeout  => "get-time: Get the timeout of the serial port.",
      CommandType::GetFlow     => "get-flow: Get the flow control of the serial port.",
      CommandType::GetInQue    => "get-in: Get the input queue of the serial port.",
      CommandType::GetOutQue   => "get-out: Get the output queue of the serial port.",
      CommandType::GetCts      => "get-cts: Get the CTS of the serial port.",
      CommandType::GetDsr      => "get-dsr: Get the DSR of the serial port.",
      CommandType::GetRi       => "get-ri: Get the RI of the serial port.",
      CommandType::GetCd       => "get-cd: Get the CD of the serial port.",
      CommandType::None        => "none: Invalid command.",
    }
  }
}
