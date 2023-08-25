mod mode;
mod util;
mod input;
mod event;
mod ending;
mod command;

use std::{
  io  ::Write,
  str ::FromStr,
  cell::RefCell,
};

use mode   ::Mode;
use regex  ::Regex;
use ending ::Ending;
use command::CommandType;

use serialport::{
  self,
  Parity,
  DataBits,
  StopBits,
  SerialPort,
  FlowControl,
  available_ports,
};

use crossterm::{
  queue,
  execute,

  cursor::{
    MoveTo,
    MoveUp,
  },

  style::{
    Color,
    Print,
    ResetColor,
    SetForegroundColor,
  },

  terminal::{
    Clear,
    SetTitle,
    ClearType,
  },
};




/* TODO

  4.   Copy & paste
  4-1. Select
  6.   Delete by word       // not possible currently due to crossterm
  7.   Calculate checksum
  8.   Reverse send
*/



const MAX_ARG_COUNT: usize = 3;

const HELP_MESSAGE: &str = "Help:
  Hot keys:
    Ctrl-C × 2: exit

  Commands:
    help             : show this
    clear            : clear screen

    send <message>   : send message
    recv             : receive message
    flush            : flush serial port

    set-mode   <mode>: set mode             mode  : ascii, hex
    set-ending <end> : set auto ending      end   : none, cr, lf, crlf
    set-rev    <rev> : set reverse send     rev   : on, off

    set-port <name>  : set port             name  : string
    set-baud <rate>  : set baud rate        rate  : 9600, 19200, 38400, 57600,
                                                    115200, or custom
    set-data <dbits> : set data bits        dbits : 5, 6, 7, 8
    set-par  <parity>: set parity           parity: none, odd, even
    set-stop <sbits> : set stop bits        sbits : 1, 2
    set-time <time>  : set timeout          time  : milliseconds
    set-flow <flow>  : set flow control     flow  : none, software, hardware

    set-rts  <state> : set RTS state        state : on, off
    set-dtr  <state> : set DTR state        state : on, off

    get-mode         : quarry mode
    get-ending       : quarry auto ending
    get-rev          : quarry reverse send

    get-port         : quarry port name
    get-baud         : quarry baud rate
    get-data         : quarry data bits
    get-par          : quarry parity
    get-stop         : quarry stop bits
    get-time         : quarry timeout
    get-flow         : quarry flow control

    get-in           : quarry input  buffer
    get-out          : quarry output buffer

    get-cts          : quarry CTS state
    get-dsr          : quarry DSR state
    get-ri           : quarry RI  state
    get-cd           : quarry CD  state

    sum <type> <data>: calculate checksum";




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

    let port_name: String;
    let baud_rate: u32;
    let data_bits: DataBits;
    let parity   : Parity  ;
    let stop_bits: StopBits;

    { // get port name
      let ports = available_ports().unwrap();

      let mut input = input::InputBuilder::new("Port Name: ")
        .preprocessor(|s, _| {
          let name = s.concat();

          let mut candidate = ports.iter()
            .map(|p| {
              let len = name.len();
              if p.port_name.len() < len ||
                !p.port_name.starts_with(&name) {
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
        .preprocessor(|s, _| {
          let rate = s.concat();

          let mut candidate = vec![
            "9600"  .to_string(),
            "19200" .to_string(),
            "38400" .to_string(),
            "57600" .to_string(),
            "115200".to_string(),
          ].into_iter()
            .filter(|s| s.starts_with(&rate))
            .map   (|s| s[rate.len()..].to_string())
            .collect::<Vec<String>>();

          candidate.retain(|s| s.len() > 0);

          input::Processed {
            buffer   : s,
            candidate,
          }
        })
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
              "5" => data_bits = DataBits::Five,
              "6" => data_bits = DataBits::Six,
              "7" => data_bits = DataBits::Seven,
              "8" => data_bits = DataBits::Eight,

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

          let mut candidate = vec![ "none", "odd", "even",]
            .into_iter()
            .filter(|s| s.starts_with(&par))
            .map(|s| s[par.len()..].to_string())
            .collect::<Vec<String>>();

          candidate.retain(|s| s.len() > 0);

          input::Processed {
            buffer   : s,
            candidate,
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
              "none" => parity = Parity::None,
              "odd"  => parity = Parity::Odd,
              "even" => parity = Parity::Even,

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
              "1" => stop_bits = StopBits::One,
              "2" => stop_bits = StopBits::Two,

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
    Print("\n\n"),
  ).unwrap();


  let mode            = RefCell::new(Mode::ASCII);
  let ctrl_c          = RefCell::new(false);
  let ending          = RefCell::new(Ending::None);
  let reverse         = RefCell::new(false);

  let candidate_states = RefCell::new([
    CandidateState::None,
    CandidateState::None,
    CandidateState::None,
  ]);


  let mut input = input::InputBuilder::new("> ")
    .preprocessor(|s, _| {
      let (parts, space_count) = split_by_space::<MAX_ARG_COUNT>(s.clone());

      let command     = parts[0].clone();
      let command_str = command.concat();

      let mut processed = Vec::<String>::new();
      let mut candidate = Vec::<String>::new();

      let mut candidate_states = candidate_states.borrow_mut();

      // processed
      match command_str.as_str() {
        "send" if *mode.borrow() == Mode::ASCII => {
          processed.extend(command);

          if space_count > 0 {
            processed.push(" ".to_string());
            processed.extend(string_to_vec_ascii(
              parts[1..].join(&" ".to_string()).concat()));
          }
        },

        "sum" if *mode.borrow() == Mode::ASCII => {
          processed.extend(command);

          if space_count > 0 {
            processed.push(" ".to_string());
            processed.extend(parts[1].clone());
          }

          if space_count > 1 {
            processed.push(" ".to_string());
            processed.extend(string_to_vec_ascii(
              parts[2..].join(&" ".to_string()).concat()));
          }
        },

        _ => {
          processed = s;
        },
      }

      // candidate
      match command_str.as_str() {
        "set-mode" => {
          candidate.push("ascii".to_string());
          candidate.push("hex"  .to_string());
        },

        "set-ending" => {
          candidate.push("none".to_string());
          candidate.push("cr"  .to_string());
          candidate.push("lf"  .to_string());
          candidate.push("crlf".to_string());
        },

        "set-port" => {
          let ports = available_ports().unwrap();

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

        "set-time" => {
          candidate.push("100" .to_string());
          candidate.push("500" .to_string());
          candidate.push("1000".to_string());
          candidate.push("2000".to_string());
        },

        "set-flow" => {
          candidate.push("none"    .to_string());
          candidate.push("software".to_string());
          candidate.push("hardware".to_string());
        },

        | "set-rev"
        | "set-rts"
        | "set-dtr" => {
          candidate.push("on" .to_string());
          candidate.push("off".to_string());
        },

        "sum" if space_count == 1 => {
          candidate.push("crc16"          .to_string());
          candidate.push("sum-byte"       .to_string());
          candidate.push("sum-byte-2comp" .to_string());
        },

        _ if space_count == 0 => {
          candidate.push("help"      .to_string());
          candidate.push("clear"     .to_string());
          candidate.push("send"      .to_string());
          candidate.push("recv"      .to_string());
          candidate.push("flush"     .to_string());
          candidate.push("set-mode"  .to_string());
          candidate.push("set-ending".to_string());
          candidate.push("set-rev"   .to_string());
          candidate.push("set-port"  .to_string());
          candidate.push("set-baud"  .to_string());
          candidate.push("set-par"   .to_string());
          candidate.push("set-data"  .to_string());
          candidate.push("set-stop"  .to_string());
          candidate.push("set-time"  .to_string());
          candidate.push("set-flow"  .to_string());
          candidate.push("set-rts"   .to_string());
          candidate.push("set-dtr"   .to_string());
          candidate.push("get-mode"  .to_string());
          candidate.push("get-ending".to_string());
          candidate.push("get-rev"   .to_string());
          candidate.push("get-port"  .to_string());
          candidate.push("get-baud"  .to_string());
          candidate.push("get-data"  .to_string());
          candidate.push("get-par"   .to_string());
          candidate.push("get-stop"  .to_string());
          candidate.push("get-time"  .to_string());
          candidate.push("get-flow"  .to_string());
          candidate.push("get-in"    .to_string());
          candidate.push("get-out"   .to_string());
          candidate.push("get-cts"   .to_string());
          candidate.push("get-dsr"   .to_string());
          candidate.push("get-ri"    .to_string());
          candidate.push("get-cd"    .to_string());
          candidate.push("sum"       .to_string());
        },

        _ => {},
      }

      // last part
      let (last_part, last_index) = get_last(&parts, space_count);
      let last_part = last_part.concat();

      // check if last part matches any candidate
      let match_any = candidate.iter().any(|s| s == &last_part);

      // filter candidate
      candidate.retain(|s|
        s.starts_with(&last_part) &&
        s.len() > last_part.len());

      // set candidate state
      candidate_states[last_index] =
             if match_any           { CandidateState::Match }
        else if candidate.len() > 0 { CandidateState::Has   }
        else                        { CandidateState::None  };

      let prefix_len = last_part.len();

      input::Processed {
        buffer   : processed,
        candidate: candidate.iter().map(|s|
          s[prefix_len..].to_string()).collect(),
      }
    })
    .renderer(|s, mut c| {
      let (parts, space_count) = split_by_space::<MAX_ARG_COUNT>(s.clone());

      let command     = parts[0].clone();
      let command_str = command.unwrap_or(Vec::new()).concat();

      let mut column    = 0usize;
      let mut processed = String::new();

      let candidate_states = candidate_states.borrow();

      // command
      match command_str.as_str() {
        | "help"
        | "clear"
        | "send"
        | "recv"
        | "flush"
        | "set-mode"
        | "set-ending"
        | "set-rev"
        | "set-port"
        | "set-baud"
        | "set-data"
        | "set-par"
        | "set-stop"
        | "set-time"
        | "set-flow"
        | "set-rts"
        | "set-dtr"
        | "get-mode"
        | "get-ending"
        | "get-rev"
        | "get-port"
        | "get-baud"
        | "get-data"
        | "get-par"
        | "get-stop"
        | "get-time"
        | "get-flow"
        | "get-in"
        | "get-out"
        | "get-cts"
        | "get-dsr"
        | "get-ri"
        | "get-cd"
        | "sum" => {
          processed.push_str(&SetForegroundColor(Color::Blue).to_string());
        },

        _ => {
          if space_count > 1 || candidate_states[0] == CandidateState::None {
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
      if space_count > 1 {
        processed.push(' ');
        column += calc_col(1);
      }

      // buffer
      match command_str.as_str() {
        // Argument must match
        | "set-mode"
        | "set-ending"
        | "set-rev"
        | "set-port"
        | "set-par"
        | "set-data"
        | "set-stop"
        | "set-flow"
        | "set-rts"
        | "set-dtr" => {
          let (buffer, _) = get_last(&parts);
          let buffer_str = buffer.concat();

          processed.push_str(&SetForegroundColor(
            match candidate_states[1] {
              CandidateState::Match
                if parts.len() < 3  => Color::Green,
              CandidateState::Has   => Color::White,
              _                     => Color::Red,
            }
          ).to_string());
          processed.push_str(&buffer_str);

          column += calc_col(buffer_str.len());
        },

        // Argument can match
        | "set-baud"
        | "set-time" => {
          let (buffer, _) = get_last(&parts);
          let buffer_str = buffer.concat();

          processed.push_str(&SetForegroundColor(
            if re_pos_int.is_match(&buffer_str) { Color::White }
            else                                { Color::Red   }
          ).to_string());
          processed.push_str(&buffer_str);

          column += calc_col(buffer_str.len());
        },

        // Argument in special format
        "send" => {
          let (buffer, last_index) = get_last(&parts);
          let buffer_str = buffer.concat();

          if last_index == 1 {
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
          }
        },

        // No argument
        _ => {
          let (buffer, _) = get_last(&parts);
          let buffer_str = buffer.concat();

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


  let read_serial = |mut port: Box<dyn SerialPort>| {
    let mut stdout = std::io::stdout();
    let mut buffer = [0u8; 1024];

    execute!(
      stdout,
      Print("Receiving..."),
    ).unwrap();

    match port.read(&mut buffer) {
      Ok(count) => {
        queue!(
          stdout,
          Print("\rRecv"),
          SetForegroundColor(Color::Green),
          Print(format!(" {:4} ", count)),
          ResetColor,
          Print("bytes: "),
        ).unwrap();

        let mut tmp = String::new();

        let mut buffer = buffer[..count].to_vec();
        if *(reverse.borrow()) {
          buffer.reverse();
        }

        match *mode.borrow() {
          Mode::ASCII => {
            for i in buffer {
              tmp.push_str(get_printable_ascii(
                if i.is_ascii_graphic() { (i as char).to_string() }
                else                    { format!("\\{:02X}", i)  },
              ).as_str());
            }
          },

          Mode::HEX => {
            for i in buffer {
              tmp.push_str(format!("{:02X}", i).as_str());
            }
          },
        }

        queue!(
          stdout,
          Print(tmp),
        ).unwrap();
      },

      Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
        queue!(
          stdout,
          SetForegroundColor(Color::Red),
          Print("\rTimed out."),
          Print(format!("({} ms)", port.timeout().as_millis())),
          ResetColor,
        ).unwrap();
      },

      _ => {
        queue!(
          stdout,
          SetForegroundColor(Color::Red),
          Print("\rFailed to receive."),
          ResetColor,
        ).unwrap();
      },
    }
  };


  loop {
    let prompt_result = input.prompt();

    if let Err(_) = prompt_result {
      let mut ctrl_c = ctrl_c.borrow_mut();

      if *ctrl_c { break; }

      queue!(
        stdout,
        SetForegroundColor(Color::Red),
        Print("\nPress again to exit.\n\n"),
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

      (CommandType::Send, _) => {
        let arg = argument.as_str();

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
          let mut buffer   = Vec::<u8>::new();
          let mut sent_str = String::new();

          // convert message to bytes
          match *mode.borrow() {
            Mode::ASCII => {
              let fragments = string_to_vec_ascii(arg.to_string());

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

              sent_str = fragments
                .iter()
                .map(|s| get_printable_ascii(s.to_string()))
                .collect::<String>();
            },

            Mode::HEX => {
              let mut tmp = arg;

              while tmp.len() > 0 {
                if let Ok(v) = u8::from_str_radix(&tmp[..2], 16) {
                  buffer.push(v);
                  tmp = &tmp[2..];

                  sent_str.push_str(format!("{:02X} ", v).as_str());
                }
              }
            },
          }

          // add ending
          match *ending.borrow() {
            Ending::None => {},
            Ending::CR   => {
              buffer.push('\r' as u8);
              sent_str.push_str(
                if *mode.borrow() == Mode::ASCII { "[CR]" }
                else                             { "0D"   });
            },
            Ending::LF   => {
              buffer.push('\n' as u8);
              sent_str.push_str(
                if *mode.borrow() == Mode::ASCII { "[LF]" }
                else                             { "0A"   });
            },
            Ending::CRLF => {
              buffer.push('\r' as u8);
              buffer.push('\n' as u8);

              sent_str.push_str(
                if *mode.borrow() == Mode::ASCII { "[CR][LF]" }
                else                             { "0D 0A"    });
            },
          }

          // send message
          match port.write({
            if *reverse.borrow() {
              buffer.reverse(); }
            &buffer
          }) {
            Ok(_) => {
              queue!(
                stdout,
                Print(
                  if *reverse.borrow() { "[Reverse]\n" }
                  else                 { ""            }
                ),
                Print("Sent "),
                SetForegroundColor(Color::Green),
                Print(format!("{:4}", buffer.len())),
                ResetColor,
                Print(" bytes: "),
                Print(sent_str),
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

          execute!(
            stdout,
            Print("\n"),
          ).unwrap();

          read_serial(port.try_clone().unwrap());
        }
      },

      (CommandType::Receive, "") =>
        (read_serial)(port.try_clone().unwrap()),

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

      (CommandType::SetEnding, arg) =>
        match arg.to_lowercase().as_str() {
          "none" => {
            *ending.borrow_mut() = Ending::None;

            queue!(
              stdout,
              Print("Ending: "),
              SetForegroundColor(Color::Green),
              Print("None"),
              ResetColor,
            ).unwrap();
          },

          "cr" => {
            *ending.borrow_mut() = Ending::CR;

            queue!(
              stdout,
              Print("Ending: "),
              SetForegroundColor(Color::Green),
              Print("CR"),
              ResetColor,
            ).unwrap();
          },

          "lf" => {
            *ending.borrow_mut() = Ending::LF;

            queue!(
              stdout,
              Print("Ending: "),
              SetForegroundColor(Color::Green),
              Print("LF"),
              ResetColor,
            ).unwrap();
          },

          "crlf" => {
            *ending.borrow_mut() = Ending::CRLF;

            queue!(
              stdout,
              Print("Ending: "),
              SetForegroundColor(Color::Green),
              Print("CRLF"),
              ResetColor,
            ).unwrap();
          },

          _ => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid ending."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::SetReverse, arg) =>
        match arg.to_lowercase().as_str() {
          "on" => {
            *(reverse.borrow_mut()) = true;

            queue!(
              stdout,
              Print("Reverse: "),
              SetForegroundColor(Color::Green),
              Print("On"),
              ResetColor,
            ).unwrap();
          }

          "off" => {
            *(reverse.borrow_mut()) = false;

            queue!(
              stdout,
              Print("Reverse: "),
              SetForegroundColor(Color::Green),
              Print("Off"),
              ResetColor,
            ).unwrap();
          }

          _ => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid reverse."),
              ResetColor,
            ).unwrap();
          },
        }

      (CommandType::SetPort, _) =>
        if let Some(_) = available_ports()
          .unwrap()
          .iter()
          .find(|p| p.port_name == argument) {

          let new_port = serialport::new(
              &argument, port.baud_rate().unwrap())
            .data_bits(port.data_bits().unwrap())
            .parity   (port.parity   ().unwrap())
            .stop_bits(port.stop_bits().unwrap())
            .timeout  (port.timeout  ()         )
            .open();

          match new_port {
            Ok(new_port) => {
              port = new_port;

              queue!(
                stdout,
                Print("Switched to port: "),
                SetForegroundColor(Color::Green),
                Print(argument),
                ResetColor,
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
            "5" => port.set_data_bits(DataBits::Five ),
            "6" => port.set_data_bits(DataBits::Six  ),
            "7" => port.set_data_bits(DataBits::Seven),
            "8" => port.set_data_bits(DataBits::Eight),

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
            "none" => port.set_parity(Parity::None),
            "odd"  => port.set_parity(Parity::Odd ),
            "even" => port.set_parity(Parity::Even),

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
            "1" => port.set_stop_bits(StopBits::One),
            "2" => port.set_stop_bits(StopBits::Two),

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

      (CommandType::SetFlow, arg) =>
        match {
          match arg.to_lowercase().as_str() {
            "none"     => port.set_flow_control(FlowControl::None    ),
            "software" => port.set_flow_control(FlowControl::Software),
            "hardware" => port.set_flow_control(FlowControl::Hardware),

            _ => Err(serialport::Error::new(
              serialport::ErrorKind::InvalidInput,
              "Invalid flow control.",
            )),
          }
        } {
          Ok(_) => {
            queue!(
              stdout,
              Print("Flow control: "),
              SetForegroundColor(Color::Green),
              Print(arg),
              ResetColor,
            ).unwrap();
          },

          Err(serialport::Error { kind: serialport::ErrorKind::InvalidInput, .. }) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Invalid flow control."),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to set flow control."),
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

      (CommandType::GetMode, "") => {
        queue!(
          stdout,
          Print("Mode: "),
          SetForegroundColor(Color::Green),
          Print(format!("{:?}", mode.borrow())),
          ResetColor,
        ).unwrap();
      },

      (CommandType::GetEnding, "") => {
        queue!(
          stdout,
          Print("Ending: "),
          SetForegroundColor(Color::Green),
          Print(format!("{:?}", ending.borrow())),
          ResetColor,
        ).unwrap();
      },

      (CommandType::GetReverse, "") => {
        queue!(
          stdout,
          Print("Reverse: "),
          SetForegroundColor(Color::Green),
          Print(
            if *reverse.borrow() { "On"  }
            else                 { "Off" }
          ),
          ResetColor,
        ).unwrap();
      },

      (CommandType::GetPort, "") => {
        queue!(
          stdout,
          Print("Port: "),
          SetForegroundColor(Color::Green),
          Print(port.name().unwrap()),
          ResetColor,
        ).unwrap();
      },

      (CommandType::GetBaud, "") => {
        queue!(
          stdout,
          Print("Baud rate: "),
          SetForegroundColor(Color::Green),
          Print(format!("{}", port.baud_rate().unwrap())),
          ResetColor,
        ).unwrap();
      },

      (CommandType::GetDataBits, "") => {
        queue!(
          stdout,
          Print("Data bits: "),
          SetForegroundColor(Color::Green),
          Print(format!("{}", port.data_bits().unwrap())),
          ResetColor,
        ).unwrap();
      },

      (CommandType::GetParity, "") => {
        queue!(
          stdout,
          Print("Parity: "),
          SetForegroundColor(Color::Green),
          Print(format!("{:?}", port.parity().unwrap())),
          ResetColor,
        ).unwrap();
      },

      (CommandType::GetStopBits, "") => {
        queue!(
          stdout,
          Print("Stop bits: "),
          SetForegroundColor(Color::Green),
          Print(format!("{:?}", port.stop_bits().unwrap())),
          ResetColor,
        ).unwrap();
      },

      (CommandType::GetTimeout, "") => {
        queue!(
          stdout,
          Print("Timeout: "),
          SetForegroundColor(Color::Green),
          Print(format!("{} ms", port.timeout().as_millis())),
          ResetColor,
        ).unwrap();
      },

      (CommandType::GetFlow, "") => {
        queue!(
          stdout,
          Print("Flow control: "),
          SetForegroundColor(Color::Green),
          Print(format!("{:?}", port.flow_control().unwrap())),
          ResetColor,
        ).unwrap();
      },

      (CommandType::GetInQue, "") =>
        match port.bytes_to_read() {
          Ok(count) => {
            queue!(
              stdout,
              Print("In: "),
              SetForegroundColor(Color::Green),
              Print(format!("{:4} bytes", count)),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to get In bytes."),
              ResetColor,
            ).unwrap();
          },
        },

      (CommandType::GetOutQue, "") =>
        match port.bytes_to_write() {
          Ok(count) => {
            queue!(
              stdout,
              Print("Out: "),
              SetForegroundColor(Color::Green),
              Print(format!("{:4} bytes", count)),
              ResetColor,
            ).unwrap();
          },

          Err(_) => {
            queue!(
              stdout,
              SetForegroundColor(Color::Red),
              Print("Failed to get Out bytes."),
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


#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum CandidateState {
  None,
  Has,
  Match,
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

    if tmp.starts_with(" ") {
      ret.push("\\20".to_string());
      tmp = tmp[1..].to_string();
      continue;
    }

    ret.push(tmp.remove(0).to_string());
  }

  ret
}


fn split_by_space<const N: usize>(s: Vec<String>) -> ([Option<Vec<String>>; N], usize) {
  let mut tmp = s
    .split(|s| s == " ")
    .map  (|s| s.to_vec())
    .collect::<Vec<Vec<String>>>();

  let mut ret = [(); N]
    .map(|_| None::<Vec<String>>);

  if tmp.len() > N {
    let rest = tmp[(N-1)..]
      .join(&" ".to_string());

    tmp[N-1] = rest;
    tmp.truncate(N);
  }

  for i in 0..N {
    ret[i] = tmp.get(i).map(|v| v.clone());
  }

  (ret, tmp.len() - 1)
}


fn get_printable_ascii(s: String) -> String {
  match s.to_ascii_lowercase().as_str() {
    | r" "
    | r"\20" => "[SP]",

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


fn get_max_arg_count(command: &String) -> usize {
  match command.as_str() {
    "sum" => 2,

    | "help"
    | "clear"
    | "flush"
    | "receive"
    | "get-mode"
    | "get-ending"
    | "get-reverse"
    | "get-port"
    | "get-baud"
    | "get-data"
    | "get-par"
    | "get-stop"
    | "get-time"
    | "get-flow"
    | "get-in"
    | "get-out"
    | "get-cts"
    | "get-dsr"
    | "get-ri"
    | "get-cd" => 0,

    _ => 1,
  }
}


fn get_last(parts: &[Option<Vec<String>>; MAX_ARG_COUNT]) -> (Vec<String>, usize) {
  if parts[0].is_none() {
    return (Vec::<String>::new(), 0);
  }

  let command = parts[0].as_ref().unwrap().concat();
  let max_arg = get_max_arg_count(&command);

  let space_count = parts.iter().position(|p| p.is_none()).unwrap() - 1;

  if space_count <= max_arg {
    return (
      parts[space_count]
        .as_ref()
        .unwrap()
        .clone(),
      space_count
    );
  }

  return (
    parts[max_arg..]
      .iter()
      .filter_map(|p| p.clone())
      .fold(Vec::new(), |mut acc, v| { acc.extend(v); acc }),
    max_arg
  );
}
