mod mode;
mod util;
mod input;
mod event;
mod command;

use std::{
  io  ::Write,
  str ::FromStr,
  cell::RefCell,
};

use mode   ::Mode;
use regex  ::Regex;
use command::CommandType;

use serialport::{
  self,
  FlowControl,
};

use crossterm::{
  queue,
  execute,

  event::{
    read,
    Event,
    MouseEvent,
    MouseEventKind,
    EnableMouseCapture,
  },

  cursor::{
    MoveTo,
    MoveLeft,
    MoveDown,
    SavePosition, RestorePosition, MoveUp, MoveToColumn,
  },

  style::{
    Color,
    Print,
    ResetColor,
    SetForegroundColor, SetBackgroundColor,
  },

  terminal::{
    Clear,
    SetTitle,
    ClearType,
  },
};




const HELP_MESSAGE: &str = "Help:
  Hot keys:
    Ctrl-C × 2: exit

  Commands:
    help             : show this
    clear            : clear screen

    send <message>   : send message
    recv             : receive message
    flush            : flush serial port

    set-mode <mode>  : set mode             mode  : ascii, hex

    set-port <name>  : set port             name  : string
    set-baud <rate>  : set baud rate        rate  : 9600, 19200, 38400, 57600,
                                                    115200, or custom
    set-data <dbits> : set data bits        dbits : 5, 6, 7, 8
    set-par  <parity>: set parity           parity: none, odd, even
    set-stop <sbits> : set stop bits        sbits : 1, 2
    set-time <time>  : set timeout          time  : milliseconds

    set-rts  <state> : set RTS state        state : on, off
    set-dtr  <state> : set DTR state        state : on, off

    get-cts          : quarry CTS state
    get-dsr          : quarry DSR state
    get-ri           : quarry RI  state
    get-cd           : quarry CD  state\n";




fn main() {
  let mut stdout = std::io::stdout();

  let re_hex     = Regex::new(r"^([0-9A-Fa-f]{2})+$"                                      ).unwrap();
  let re_ascii   = Regex::new(r"^((\\\\)|(\\[01][0-9A-Fa-f])|(\\7[fF])|([\ -~&&[^\\]]))+$").unwrap();
  let re_pos_int = Regex::new(r"^[1-9][0-9]*$"                                            ).unwrap();


  // clear screen
  execute!(
    stdout,
    Clear(ClearType::All),
    MoveTo(0, 0),
    SetTitle("Serial Tester"),
  ).unwrap();


  // Get first serial port
  let mut port = {
    execute!(
      stdout,
      Print("Set the serial port.\n\n"),
    ).unwrap();

    let mut port_name = String::new();
    let mut baud_rate = 19200;
    let mut data_bits = serialport::DataBits::Eight;
    let mut parity    = serialport::Parity  ::None;
    let mut stop_bits = serialport::StopBits::One;

    { // get port name
      let ports = serialport::available_ports().unwrap();

      let mut input = input::InputBuilder::new("Port Name: ")
        .preprocessor(|s, _| {
          let name = s.concat();

          let mut candidate = ports.iter()
            .map(|p| {
              let len = name.len();
              if p.port_name.len() < len {
                return String::new(); }
              p.port_name[len..].to_string()
            })
            .collect::<Vec<String>>();

          candidate.retain(|s| s.len() > 0);

          input::Processed {
            buffer   : s,
            candidate: candidate,
          }
        })
        .renderer(|s, c| {
          let mut processed = String::new();

          let name = s.concat();

          if ports.iter().any(|p| p.port_name == name) {
            processed.push_str(&SetForegroundColor(Color::Green).to_string());
          }

          else if !ports.iter().any(|p| p.port_name.starts_with(&name)) {
            processed.push_str(&SetForegroundColor(Color::Red).to_string());
          }

          processed.push_str(&name);

          (processed, c)
        })
        .build();

      loop {
        match input.prompt() {
          Ok(result) => {
            if ports.iter().any(|p| p.port_name == result) {
              port_name = result;
              break;
            }

            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid port name.\r"),
              ResetColor,
              MoveUp(1),
            ).unwrap();
          },

          Err(_) => {
            panic!("Keyboard interrupt.");
          },
        }
      }
    }

    { // get baud rate
      let mut input = input::InputBuilder::new("Baud Rate: ")
        .renderer(|s, c| {
          let mut processed = String::new();

          let rate = s.concat();

          if let Err(_) = u32::from_str(&rate) {
            processed.push_str(&SetForegroundColor(Color::Red).to_string());
          }

          processed.push_str(&rate);

          (processed, c)
        })
        .build_with_final(|s| u32::from_str(&s));

      loop {
        match input.prompt() {
          Ok(Ok(rate)) => {
            baud_rate = rate;
            break;
          },

          Ok(Err(_)) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid baud rate.\r"),
              MoveUp(1),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            panic!("Keyboard interrupt.");
          },
        }
      }
    }

    { // get data bits
      let mut input = input::InputBuilder::new("Data bits: ")
        .preprocessor(|s, _| {
          let bits = s.concat();

          input::Processed {
            buffer   : s,
            candidate:
              if bits.len() != 0 { Vec::new() }
              else { vec![
                "8".to_string(),
                "7".to_string(),
                "6".to_string(),
                "5".to_string(),
              ]},
          }
        })
        .renderer(|s, c| {
          let mut processed = String::new();

          let bits = s.concat();

          match bits.as_str() {
            | "5"
            | "6"
            | "7"
            | "8" => {
              processed.push_str(&SetForegroundColor(Color::Green).to_string());
            },

            _ => {
              processed.push_str(&SetForegroundColor(Color::Red).to_string());
            },
          }

          processed.push_str(&bits);

          (processed, c)
        })
        .build();

      loop {
        match input.prompt() {
          Ok(bit) => {
            match bit.as_str() {
              "5" => data_bits = serialport::DataBits::Five,
              "6" => data_bits = serialport::DataBits::Six,
              "7" => data_bits = serialport::DataBits::Seven,
              "8" => data_bits = serialport::DataBits::Eight,

              _ => {
                queue!(
                  stdout,
                  SetForegroundColor(Color::Red),
                  Print("Invalid data bits.\r"),
                  MoveUp(1),
                  ResetColor,
                ).unwrap();

                continue;
              },
            }

            break;
          },

          Err(_) => {
            panic!("Keyboard interrupt.");
          },
        }
      }
    }

    { // get parity
      let mut input = input::InputBuilder::new("Parity   : ")
        .preprocessor(|s, _| {
          let par = s.concat();

          input::Processed {
            buffer   : s,
            candidate: vec![ "none", "odd", "even",]
              .into_iter()
              .filter(|s| s.starts_with(&par))
              .map(|s| s[par.len()..].to_string())
              .collect::<Vec<String>>(),
          }
        })
        .renderer(|s, c| {
          let mut processed = String::new();

          let parity = s.concat();

          match parity.as_str() {
            | "none"
            | "odd"
            | "even" => {
              processed.push_str(&SetForegroundColor(Color::Green).to_string());
            },

            _ => {
              processed.push_str(&SetForegroundColor(Color::Red).to_string());
            },
          }

          processed.push_str(&parity);

          (processed, c)
        })
        .build();

      loop {
        match input.prompt() {
          Ok(par) => {
            match par.as_str() {
              "none" => parity = serialport::Parity::None,
              "odd"  => parity = serialport::Parity::Odd,
              "even" => parity = serialport::Parity::Even,

              _ => {
                queue!(
                  stdout,
                  SetForegroundColor(Color::Red),
                  Print("Invalid parity.\r"),
                  MoveUp(1),
                  ResetColor,
                ).unwrap();

                continue;
              },
            }

            break;
          },

          Err(_) => {
            panic!("Keyboard interrupt.");
          },
        }
      }
    }

    { // get stop bits
      let mut input = input::InputBuilder::new("Stop bits: ")
        .preprocessor(|s, _| {
          let bits = s.concat();

          input::Processed {
            buffer   : s,
            candidate:
              if bits.len() != 0 { Vec::new() }
              else { vec![
                "1".to_string(),
                "2".to_string(),
              ]},
          }
        })
        .renderer(|s, c| {
          let mut processed = String::new();

          let bits = s.concat();

          match bits.as_str() {
            | "1"
            | "2" => {
              processed.push_str(&SetForegroundColor(Color::Green).to_string());
            },

            _ => {
              processed.push_str(&SetForegroundColor(Color::Red).to_string());
            },
          }

          processed.push_str(&bits);

          (processed, c)
        })
        .build();

      loop {
        match input.prompt() {
          Ok(bit) => {
            match bit.as_str() {
              "1" => stop_bits = serialport::StopBits::One,
              "2" => stop_bits = serialport::StopBits::Two,

              _ => {
                queue!(
                  stdout,
                  SetForegroundColor(Color::Red),
                  Print("Invalid data bits.\r"),
                  MoveUp(1),
                  ResetColor,
                ).unwrap();

                continue;
              },
            }

            break;
          },

          Err(_) => {
            panic!("Keyboard interrupt.");
          },
        }
      }
    }

    match serialport::new(&port_name, baud_rate)
      .data_bits(data_bits)
      .parity   (parity   )
      .stop_bits(stop_bits)
      .timeout  (std::time::Duration::from_millis(100))
      .open()
    {
      Ok(port) => port,
      Err(_) => {
        panic!("Failed to open serial port.");
      },
    }
  };


  // print help message
  execute!(
    stdout,
    Clear(ClearType::All),
    MoveTo(0, 0),
    Print(HELP_MESSAGE),
  ).unwrap();


  let mode            = RefCell::new(Mode::ASCII);
  let ctrl_c          = RefCell::new(false);
  let has_candidate   = RefCell::new(false);
  let match_candidate = RefCell::new(false);


  let mut input = input::InputBuilder::new("> ")
    .preprocessor(|s, _| {
      let (command, buffer) = split_cmd_and_buf(s.clone());

      let buffer_str  = buffer .concat();
      let command_str = command.concat();

      let mut processed = Vec::<String>::new();
      let mut candidate = Vec::<String>::new();

      let has_space = s.len() > command_str.len();

      let mut has_candidate   = has_candidate  .borrow_mut();
      let mut match_candidate = match_candidate.borrow_mut();

      // processed
      match command_str.as_str() {
        "send" if *mode.borrow() == Mode::ASCII => {
          processed.extend(command);

          if has_space {
          processed.push(" ".to_string());
          processed.extend(string_to_vec_ascii(buffer.concat()));
          }
        },

        _ => {
          processed = s;
        },
      }

      // candidate
      if processed.len() > 0 {
        match command_str.as_str() {
          "set-mode" => {
            candidate.push("ascii".to_string());
            candidate.push("hex"  .to_string());
          },

          "set-port" => {
            let ports = serialport::available_ports().unwrap();

            candidate.extend(ports.iter()
              .map(|p| p.port_name.clone()));
          },

          "set-baud" => {
            candidate.push("9600"  .to_string());
            candidate.push("19200" .to_string());
            candidate.push("38400" .to_string());
            candidate.push("57600" .to_string());
            candidate.push("115200".to_string());
          },

          "set-par" => {
            candidate.push("none".to_string());
            candidate.push("odd" .to_string());
            candidate.push("even".to_string());
          },

          "set-data" => {
            candidate.push("5".to_string());
            candidate.push("6".to_string());
            candidate.push("7".to_string());
            candidate.push("8".to_string());
          },

          "set-stop" => {
            candidate.push("1".to_string());
            candidate.push("2".to_string());
          },

          "set-rts" => {
            candidate.push("on" .to_string());
            candidate.push("off".to_string());
          },

          "set-dtr" => {
            candidate.push("on" .to_string());
            candidate.push("off".to_string());
          },

          _ => {
            if !has_space {
              candidate.push("help"    .to_string());
              candidate.push("clear"   .to_string());
              candidate.push("send"    .to_string());
              candidate.push("recv"    .to_string());
              candidate.push("flush"   .to_string());
              candidate.push("set-mode".to_string());
              candidate.push("set-port".to_string());
              candidate.push("set-baud".to_string());
              candidate.push("set-par" .to_string());
              candidate.push("set-data".to_string());
              candidate.push("set-stop".to_string());
              candidate.push("set-time".to_string());
              candidate.push("set-rts" .to_string());
              candidate.push("set-dtr" .to_string());
              candidate.push("get-cts" .to_string());
              candidate.push("get-dsr" .to_string());
              candidate.push("get-ri"  .to_string());
              candidate.push("get-cd"  .to_string());
            }
          },
        }
      }

      let prefix =
        if has_space { buffer_str .clone() }
        else         { command_str.clone() };

      *match_candidate = candidate.iter()
        .any(|s| s == &prefix);

      candidate.retain(|s|
        s.starts_with(&prefix) &&
        s.len() > prefix.len());

      *has_candidate = candidate.len() > 0;

      let prefix_len = prefix.len();

      input::Processed {
        buffer   : processed,
        candidate: candidate.iter().map(|s|
          s[prefix_len..].to_string()).collect(),
      }
    })
    .renderer(|s, c| {
      let mut c = c;

      let (command, buffer) = split_cmd_and_buf(s.clone());

      let buffer_str  = buffer .concat();
      let command_str = command.concat();

      let mut processed = String::new();
      let mut column    = 0usize;

      let has_space = s.len() > command_str.len();

      let has_candidate   = has_candidate  .borrow_mut();
      let match_candidate = match_candidate.borrow_mut();

      // command
      match command_str.as_str() {
        | "help"
        | "clear"
        | "send"
        | "recv"
        | "flush"
        | "set-mode"
        | "set-port"
        | "set-baud"
        | "set-data"
        | "set-par"
        | "set-stop"
        | "set-time"
        | "set-rts"
        | "set-dtr"
        | "get-cts"
        | "get-dsr"
        | "get-ri"
        | "get-cd" => {
          processed.push_str(&SetForegroundColor(Color::Blue).to_string());
        },

        _ => {
          if has_space {
            processed.push_str(&SetForegroundColor(Color::Red).to_string());
          }
        },
      }

      processed.push_str(&command_str);
      processed.push_str(&ResetColor.to_string());

      let mut calc_col = |len: usize| {
        if c == 0 { return 0; }

        let mut ret = len;
        if ret > c { ret = c; }

        c -= ret;

        ret
      };

      column += calc_col(command_str.len());

      // space
      if has_space {
        processed.push(' ');
        column += calc_col(1);
      }

      // buffer
      match command_str.as_str() {
        | "set-mode"
        | "set-port"
        | "set-par"
        | "set-data"
        | "set-stop"
        | "set-rts"
        | "set-dtr" => {
          processed.push_str(&SetForegroundColor(
                 if *match_candidate { Color::Green }
            else if *has_candidate   { Color::White }
            else                     { Color::Red   }
          ).to_string());
          processed.push_str(&buffer_str);

          column += calc_col(buffer_str.len());
        },

        | "set-baud"
        | "set-time" => {
          processed.push_str(&SetForegroundColor(
            if re_pos_int.is_match(&buffer_str) { Color::White }
            else                                { Color::Red   }
          ).to_string());
          processed.push_str(&buffer_str);

          column += calc_col(buffer_str.len());
        },

        "send" => {
          match *mode.borrow() {
            Mode::ASCII => {
              processed.push_str(&SetForegroundColor(
                if re_ascii.is_match(&buffer_str) { Color::White }
                else                              { Color::Red   }
              ).to_string());

              for i in buffer {
                let ch = get_printable_ascii(i);
                processed.push_str(ch.as_str());

                if c > 0 {
                  c      -= 1;
                  column += ch.len();
                }
              }
            },

            Mode::HEX => {
              processed.push_str(&SetForegroundColor(
                if re_hex.is_match(&buffer_str) { Color::White }
                else                            { Color::Red   }
              ).to_string());
              processed.push_str(&buffer_str);

              column += calc_col(buffer_str.len());
            },
          }
        },

        _ => {
          processed.push_str(&SetForegroundColor(Color::Red).to_string());
          processed.push_str(&buffer_str);

          column += calc_col(buffer_str.len());
        },
      }

      (processed, column)
    })
    .build_with_final(|s| {
      let (command_str, buffer_str) = s.split_at(s.find(' ').unwrap_or(s.len()));
      (
        CommandType::from_str(command_str).unwrap(),
        if buffer_str.len() > 0 { buffer_str[1..].to_string() }
        else                    { ""             .to_string() }
      )
    });

  loop {
    let prompt_result = input.prompt();

    if let Err(_) = prompt_result {
      let mut ctrl_c = ctrl_c.borrow_mut();

      if *ctrl_c { break; }

      queue!(
        stdout,
        SetForegroundColor(Color::Red),
        Print("Press again to exit.\n"),
        ResetColor,
      ).unwrap();

      *ctrl_c = true;

      continue;
    }

    (*ctrl_c.borrow_mut()) = false;

    let (command, argument) = prompt_result.unwrap();

    match (command, argument.to_lowercase().as_str()) {
      (CommandType::Help, "") => {
        queue!(
          stdout,
          Print(HELP_MESSAGE),
        ).unwrap();
      },

      (CommandType::Clear, "") => {
        queue!(
          stdout,
          Clear(ClearType::All),
          MoveTo(0, 0),
        ).unwrap();
      },

      (CommandType::Send, arg) => {
        // check if message is valid
        if {
          match *mode.borrow() {
            Mode::ASCII => !re_ascii.is_match(arg),
            Mode::HEX   => !re_hex  .is_match(arg),
          }
        } {
          queue!(
            stdout,
            SetForegroundColor(Color::Red),
            Print("Invalid message."),
            ResetColor,
          ).unwrap();
        }

        else {
          match *mode.borrow() {
            Mode::ASCII => {
              let mut buffer = Vec::<u8>::new();
              let fragments = string_to_vec_ascii(arg.to_string());

              // convert message to bytes
              let mut was_slash = false;

              for i in &fragments {
                if i == "\\" {
                  if !was_slash {
                    was_slash = true;
                    buffer.push('\\' as u8);
                    continue;
                  }

                  was_slash = false;
                  continue;
                }

                if i.starts_with("\\") {
                  if let Ok(v) = u8::from_str_radix(&i[1..], 16) {
                    buffer.push(v);
                  }

                  continue;
                }

                buffer.push(i.as_bytes()[0]);
              }

              // send message
              match port.write(&buffer) {
                Ok(_) => {
                  queue!(
                    stdout,
                    Print("Sent "),
                    SetForegroundColor(Color::Green),
                    Print(format!("{:4}", buffer.len())),
                    ResetColor,
                    Print(" bytes: "),
                    Print(fragments
                      .iter()
                      .map(|s| get_printable_ascii(s.to_string()))
                      .collect::<String>()),
                  ).unwrap();
                },

                Err(_) => {
                  queue!(
                    stdout,
                    SetForegroundColor(Color::Red),
                    Print("Failed to send."),
                    ResetColor,
                  ).unwrap();
                },
              }
            },

            Mode::HEX => {
              let mut buffer = Vec::<u8>::new();

              // convert message to bytes
              let mut tmp = arg;

              while tmp.len() > 0 {
                if let Ok(v) = u8::from_str_radix(&tmp[..2], 16) {
                  buffer.push(v);
                  tmp = &tmp[2..];
                }
              }

              // send message
              match port.write(&buffer) {
                Ok(_) => {
                  queue!(
                    stdout,
                    Print("Sent "),
                    SetForegroundColor(Color::Green),
                    Print(format!("{:4}", buffer.len())),
                    ResetColor,
                    Print(" bytes: "),
                    Print(arg),
                  ).unwrap();
                },

                Err(_) => {
                  queue!(
                    stdout,
                    SetForegroundColor(Color::Red),
                    Print("Failed to send."),
                    ResetColor,
                  ).unwrap();
                },
              }
            },
          }

          port.flush().unwrap();
        }
      },

      (CommandType::Receive, "") => {
        let mut buffer = [0u8; 1024];

        match port.read(&mut buffer) {
          Ok(count) => {
            queue!(
              stdout,
              Print(format!("Received {:4} bytes: ", count)),
            ).unwrap();

            let mut tmp = String::new();

            match *mode.borrow() {
              Mode::ASCII => {
                for i in buffer {
                  tmp.push_str(get_printable_ascii(
                    format!("\\{:02X}", i)
                  ).as_str());
                }
              },

              Mode::HEX => {
                for i in buffer {
                  tmp.push_str(format!("{:02X}", i).as_str());
                }
              },
            }
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to receive."),
              ResetColor,
            ).unwrap();
          },
        }
      },

      (CommandType::Flush, "") =>
        match port.flush() {
          Ok(_) => {
            queue!(
              stdout,
              Print("Flushed."),
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to flush."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::SetMode, arg) =>
        match arg.to_lowercase().as_str() {
          "ascii" => {
            *mode.borrow_mut() = Mode::ASCII;

            queue!(
              stdout,
              Print("Mode: "),
              SetForegroundColor(Color::Green),
              Print("ASCII"),
              ResetColor,
            ).unwrap();
          },

          "hex" => {
            *mode.borrow_mut() = Mode::HEX;

            queue!(
              stdout,
              Print("Mode: "),
              SetForegroundColor(Color::Green),
              Print("HEX"),
              ResetColor,
            ).unwrap();
          },

          _ => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid mode."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::SetPort, arg) =>
        if let Some(port_) = serialport::available_ports()
          .unwrap()
          .iter()
          .find(|p| p.port_name == arg) {

          let new_port = serialport::new(&port_.port_name, port.baud_rate().unwrap())
            .flow_control(FlowControl::Hardware)
            .data_bits   (port.data_bits().unwrap())
            .parity      (port.parity   ().unwrap())
            .stop_bits   (port.stop_bits().unwrap())
            .timeout     (port.timeout  ()         )
            .open();

          match new_port {
            Ok(new_port) => {
              port = new_port;

              queue!(
                stdout,
                Print("Port "),
                SetForegroundColor(Color::Green),
                Print(arg),
                ResetColor,
                Print(" is now opened."),
              ).unwrap();
            },

            Err(_) => {
              queue!(
                stdout,
                SetForegroundColor(Color::Red),
                Print("Failed to open port."),
                ResetColor,
              ).unwrap();
            },
          }
        }

        else {
          queue!(
            stdout,
            SetForegroundColor(Color::Red),
            Print("Invalid port."),
            ResetColor,
          ).unwrap();
        }

      (CommandType::SetBaud, arg) =>
        match {
          if let Ok(rate) = u32::from_str(&arg) {
            port.set_baud_rate(rate)
          }

          else {
            Err(serialport::Error::new(
              serialport::ErrorKind::InvalidInput,
              "Invalid baud rate.",
            ))
          }
        } {
          Ok(_) => {
            queue!(
              stdout,
              Print("Baud rate: "),
              SetForegroundColor(Color::Green),
              Print(arg),
              ResetColor,
            ).unwrap();
          },

          Err(serialport::Error { kind: serialport::ErrorKind::InvalidInput, .. }) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid baud rate."),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to set baud rate."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::SetDataBits, arg) =>
        match {
          match arg {
            "5" => port.set_data_bits(serialport::DataBits::Five ),
            "6" => port.set_data_bits(serialport::DataBits::Six  ),
            "7" => port.set_data_bits(serialport::DataBits::Seven),
            "8" => port.set_data_bits(serialport::DataBits::Eight),

            _ => Err(serialport::Error::new(
              serialport::ErrorKind::InvalidInput,
              "Invalid data bits.",
            )),
          }
        } {
          Ok(_) => {
            queue!(
              stdout,
              Print("Data bits: "),
              SetForegroundColor(Color::Green),
              Print(arg),
              ResetColor,
            ).unwrap();
          },

          Err(serialport::Error { kind: serialport::ErrorKind::InvalidInput, .. }) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid data bits."),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to set data bits."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::SetParity, arg) =>
        match {
          match arg.to_lowercase().as_str() {
            "none" => port.set_parity(serialport::Parity::None),
            "odd"  => port.set_parity(serialport::Parity::Odd ),
            "even" => port.set_parity(serialport::Parity::Even),

            _ => Err(serialport::Error::new(
              serialport::ErrorKind::InvalidInput,
              "Invalid parity.",
            )),
          }
        } {
          Ok(_) => {
            queue!(
              stdout,
              Print("Parity: "),
              SetForegroundColor(Color::Green),
              Print(arg),
              ResetColor,
            ).unwrap();
          },

          Err(serialport::Error { kind: serialport::ErrorKind::InvalidInput, .. }) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid parity."),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to set parity."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::SetStopBits, arg) =>
        match {
          match arg {
            "1" => port.set_stop_bits(serialport::StopBits::One),
            "2" => port.set_stop_bits(serialport::StopBits::Two),

            _ => Err(serialport::Error::new(
              serialport::ErrorKind::InvalidInput,
              "Invalid stop bits.",
            )),
          }
        } {
          Ok(_) => {
            queue!(
              stdout,
              Print("Stop bits: "),
              SetForegroundColor(Color::Green),
              Print(arg),
              ResetColor,
            ).unwrap();
          },

          Err(serialport::Error { kind: serialport::ErrorKind::InvalidInput, .. }) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid stop bits."),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to set stop bits."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::SetTimeout, arg) =>
        match {
          if let Ok(time) = u64::from_str(&arg) {
            port.set_timeout(std::time::Duration::from_millis(time))
          }

          else {
            Err(serialport::Error::new(
              serialport::ErrorKind::InvalidInput,
              "Invalid timeout.",
            ))
          }
        } {
          Ok(_) => {
            queue!(
              stdout,
              Print("Timeout: "),
              SetForegroundColor(Color::Green),
              Print(arg),
              ResetColor,
            ).unwrap();
          },

          Err(serialport::Error { kind: serialport::ErrorKind::InvalidInput, .. }) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid timeout."),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to set timeout."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::SetRts, arg) =>
        match {
          match arg.to_lowercase().as_str() {
            "on"  => port.write_request_to_send(true ),
            "off" => port.write_request_to_send(false),

            _ => Err(serialport::Error::new(
              serialport::ErrorKind::InvalidInput,
              "Invalid RTS state.",
            )),
          }
        } {
          Ok(_) => {
            queue!(
              stdout,
              Print("RTS: "),
              SetForegroundColor(Color::Green),
              Print(arg),
              ResetColor,
            ).unwrap();
          },

          Err(serialport::Error { kind: serialport::ErrorKind::InvalidInput, .. }) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid RTS state."),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to set RTS state."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::SetDtr, arg) =>
        match {
          match arg.to_lowercase().as_str() {
            "on"  => port.write_data_terminal_ready(true ),
            "off" => port.write_data_terminal_ready(false),

            _ => Err(serialport::Error::new(
              serialport::ErrorKind::InvalidInput,
              "Invalid DTR state.",
            )),
          }
        } {
          Ok(_) => {
            queue!(
              stdout,
              Print("DTR: "),
              SetForegroundColor(Color::Green),
              Print(arg),
              ResetColor,
            ).unwrap();
          },

          Err(serialport::Error { kind: serialport::ErrorKind::InvalidInput, .. }) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid DTR state."),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to set DTR state."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::GetCts, "") =>
        match port.read_clear_to_send() {
          Ok(state) => {
            queue!(
              stdout,
              Print("CTS: "),
              Print(
                if state { "On" }
                else     { "Off" }
              ),
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to get CTS state."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::GetDsr, "") =>
        match port.read_data_set_ready() {
          Ok(state) => {
            queue!(
              stdout,
              Print("DSR: "),
              Print(
                if state { "On" }
                else     { "Off" }
              ),
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to get DSR state."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::GetRi, "") =>
        match port.read_ring_indicator() {
          Ok(state) => {
            queue!(
              stdout,
              Print("RI: "),
              Print(
                if state { "On" }
                else     { "Off" }
              ),
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to get RI state."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::GetCd, "") =>
        match port.read_carrier_detect() {
          Ok(state) => {
            queue!(
              stdout,
              Print("CD: "),
              Print(
                if state { "On" }
                else     { "Off" }
              ),
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to get CD state."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::None, _) => {
        queue!(
          stdout,
          SetForegroundColor(Color::Red),
          Print("Invalid command."),
          ResetColor,
        ).unwrap();
      },

      _ => {
        queue!(
          stdout,
          SetForegroundColor(Color::Red),
          Print("Invalid argument."),
          ResetColor,
        ).unwrap();
      },
    }

    execute!(
      stdout,
      Print("\n\n"),
    ).unwrap();
  }
}


fn string_to_vec_ascii(s: String) -> Vec<String> {
  let mut tmp = s.clone();
  let mut ret = Vec::<String>::new();

  while tmp.len() > 0 {
    if tmp.starts_with("\\\\") {
      ret.push("\\".to_string());
      ret.push("\\".to_string());
      tmp = tmp[2..].to_string();
      continue;
    }

    if tmp.starts_with("\\") && tmp.len() > 2 {
      if let Ok(v) = u8::from_str_radix(&tmp[1..3], 16) {
        if v < ' ' as u8 || v == 127 {
          ret.push(tmp[..3].to_string());
          tmp = tmp[3..].to_string();
          continue;
        }
      }
    }

    ret.push(tmp.remove(0).to_string());
  }

  ret
}


fn split_cmd_and_buf(s: Vec<String>) -> (Vec<String>, Vec<String>) {
  if let Some(index) = s.iter().position(|s| s == " ") {
    let (cmd, arg) = s.split_at(index);
    (cmd.to_vec(), arg[1..].to_vec())
  }

  else {
    (s.clone(), Vec::new())
  }
}


fn get_printable_ascii(s: String) -> String {
  match s.to_ascii_lowercase().as_str() {
    r"\00" => "[NUL]",
    r"\01" => "[SOH]",
    r"\02" => "[STX]",
    r"\03" => "[ETX]",
    r"\04" => "[EOT]",
    r"\05" => "[ENQ]",
    r"\06" => "[ACK]",
    r"\07" => "[BEL]",
    r"\08" => "[BS]",
    r"\09" => "[HT]",
    r"\0a" => "[LF]",
    r"\0b" => "[VT]",
    r"\0c" => "[FF]",
    r"\0d" => "[CR]",
    r"\0e" => "[SO]",
    r"\0f" => "[SI]",
    r"\10" => "[DLE]",
    r"\11" => "[DC1]",
    r"\12" => "[DC2]",
    r"\13" => "[DC3]",
    r"\14" => "[DC4]",
    r"\15" => "[NAK]",
    r"\16" => "[SYN]",
    r"\17" => "[ETB]",
    r"\18" => "[CAN]",
    r"\19" => "[EM]",
    r"\1a" => "[SUB]",
    r"\1b" => "[ESC]",
    r"\1c" => "[FS]",
    r"\1d" => "[GS]",
    r"\1e" => "[RS]",
    r"\1f" => "[US]",
    r"\7f" => "[DEL]",
    _      => &s,
  }.to_string()
}
