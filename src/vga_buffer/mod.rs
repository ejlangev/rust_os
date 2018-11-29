use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

lazy_static! {
  pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
    column_position: 0,
    row_position: 0,
    color_code: ColorCode::new(Color::Yellow, Color::Black),
    buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
  });
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
  Black = 0,
  Blue = 1,
  Green = 2,
  Cyan = 3,
  Red = 4,
  Magenta = 5,
  Brown = 6,
  LightGray = 7,
  DarkGray = 8,
  LightBlue = 9,
  LightGreen = 10,
  LightCyan = 11,
  LightRed = 12,
  Pink = 13,
  Yellow = 14,
  White = 15,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugLevel {
  Core = 0,
  Process = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ColorCode(u8);

impl ColorCode {
  fn new(foreground: Color, background: Color) -> ColorCode {
    ColorCode((background as u8) << 4 | (foreground as u8 ))
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
  ascii_character: u8,
  color_code: ColorCode
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

struct Buffer {
  chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
  row_position: usize,
  column_position: usize,
  color_code: ColorCode,
  buffer: &'static mut Buffer,
}

impl Writer {
  fn with_color(&mut self, color: ColorCode, f: &Fn(&mut Self)) {
    let tmp_color = self.color_code;
    self.color_code = color;
    f(self);
    self.color_code = tmp_color;
  }

  pub fn write_string(&mut self, s: &str) {
    for byte in s.bytes() {
      match byte {
        // Only allow writing actual ascii characters
        0x20...0x7e | b'\n' => self.write_byte(byte),
        // Otherwise write a specific character
        _ => self.write_byte(0xfe)
      }
    }
  }

  pub fn write_byte(&mut self, byte: u8) {
    match byte {
      b'\n' => self.new_line(),
      byte => {
        if self.column_position >= BUFFER_WIDTH {
          self.new_line();
        }

        let row = self.row_position;
        let col = self.column_position;

        let color_code = self.color_code;
        self.buffer.chars[row][col].write(ScreenChar {
          ascii_character: byte,
          color_code,
        });
        self.column_position += 1;
      }
    }
  }

  pub fn clear_screen(&mut self) {
    for row in 0..BUFFER_HEIGHT {
      self.clear_row(row);
    }
    self.row_position = 0;
    self.column_position = 0;
  }

  fn new_line(&mut self) {
    if self.row_position == BUFFER_HEIGHT - 1 {
      for row in 1..BUFFER_HEIGHT {
        for col in 0..BUFFER_WIDTH {
          let character = self.buffer.chars[row][col].read();
          self.buffer.chars[row - 1][col].write(character);
        }
      }
      self.clear_row(BUFFER_HEIGHT - 1);
    } else {
      self.row_position += 1;
    }

    self.column_position = 0;
  }

  fn clear_row(&mut self, row: usize) {
    let blank = ScreenChar {
      ascii_character: b' ',
      color_code: self.color_code,
    };

    for col in 0..BUFFER_WIDTH {
      self.buffer.chars[row][col].write(blank);
    }
  }
}

impl fmt::Write for Writer {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    self.write_string(s);
    Ok(())
  }
}

#[macro_export]
macro_rules! clear_screen {
  () => ($crate::vga_buffer::_clear_screen());
}

/// Like the `print!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! print {
  ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! println {
  () => ($crate::print!("\n"));
  ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! debug {
  ($level:expr, $($arg:tt)*) => {
    $crate::vga_buffer::_debug($level, format_args!("{}\n", format_args!($($arg)*)));
  }
}

/// Prints the given formatted string to the VGA text buffer through the global `WRITER` instance.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
  use core::fmt::Write;
  WRITER.lock().write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _clear_screen() {
  WRITER.lock().clear_screen();
}

#[doc(hidden)]
pub fn _debug(level: DebugLevel, args: fmt::Arguments) {
  use core::fmt::Write;
  let color = match level {
    DebugLevel::Core => ColorCode::new(Color::Red, Color::Black),
    DebugLevel::Process => ColorCode::new(Color::Cyan, Color::Black),
  };

  WRITER.lock().with_color(color, &|writer| writer.write_fmt(args).unwrap());
}

#[cfg(test)]
mod test {
  use super::*;

  fn construct_writer() -> Writer {
    use std::boxed::Box;

    let buffer = construct_buffer();
    Writer {
      column_position: 0,
      row_position: 0,
      color_code: ColorCode::new(Color::Blue, Color::Magenta),
      buffer: Box::leak(Box::new(buffer)),
    }
  }

  fn construct_buffer() -> Buffer {
    use array_init::array_init;

    Buffer {
      chars: array_init(|_| array_init(|_| Volatile::new(empty_char()))),
    }
  }

  fn empty_char() -> ScreenChar {
    ScreenChar {
      ascii_character: b' ',
      color_code: ColorCode::new(Color::Green, Color::Brown),
    }
  }

  #[test]
  fn write_bytes() {
    let mut writer = construct_writer();
    writer.write_byte(b'X');
    writer.write_byte(b'Y');

    assert_eq!(writer.row_position, 0);
    assert_eq!(writer.column_position, 2);
    assert_eq!(writer.buffer.chars[0][0].read().ascii_character, b'X');
    assert_eq!(writer.buffer.chars[0][0].read().color_code, writer.color_code);
    assert_eq!(writer.buffer.chars[0][1].read().ascii_character, b'Y');
    assert_eq!(writer.buffer.chars[0][1].read().color_code, writer.color_code);
  }

  #[test]
  fn clear_screen() {
    let mut writer = construct_writer();
    writer.write_byte(b'X');
    writer.write_byte(b'\n');
    writer.write_byte(b'Y');

    assert_eq!(writer.row_position, 1);
    assert_eq!(writer.column_position, 1);

    writer.clear_screen();
    assert_eq!(writer.row_position, 0);
    assert_eq!(writer.column_position, 0);
    assert_eq!(writer.buffer.chars[0][0].read().ascii_character, b' ');
  }

  #[test]
  fn with_color() {
    let mut writer = construct_writer();
    let original_color = writer.color_code;
    let new_color = ColorCode::new(Color::Red, Color::Cyan);
    writer.with_color(new_color, &|writ| writ.write_byte(b'X'));
    writer.write_byte(b'Y');
    assert_eq!(writer.color_code, original_color);
    assert_eq!(writer.buffer.chars[0][0].read().ascii_character, b'X');
    assert_eq!(writer.buffer.chars[0][0].read().color_code, new_color);
    assert_eq!(writer.buffer.chars[0][1].read().ascii_character, b'Y');
    assert_eq!(writer.buffer.chars[0][1].read().color_code, original_color);
  }

  #[test]
  fn write_formatted() {
    use core::fmt::Write;

    let mut writer = construct_writer();
    writeln!(&mut writer, "a").unwrap();
    writeln!(&mut writer, "b{}", "c").unwrap();

    assert_eq!(writer.row_position, 2);
    assert_eq!(writer.column_position, 0);
    assert_eq!(writer.buffer.chars[0][0].read().ascii_character, b'a');
    assert_eq!(writer.buffer.chars[1][0].read().ascii_character, b'b');
    assert_eq!(writer.buffer.chars[1][1].read().ascii_character, b'c');
  }
}
