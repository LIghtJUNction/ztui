//! A caller-owned fixed buffer; no terminal, allocator, or Unicode crate is needed
//! by the library. This example uses std only to print the result.
use core::fmt;
use ztui::Row;

struct Buffer {
    bytes: [u8; 128],
    used: usize,
}
impl fmt::Write for Buffer {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.used.checked_add(text.len()).ok_or(fmt::Error)?;
        let target = self.bytes.get_mut(self.used..end).ok_or(fmt::Error)?;
        target.copy_from_slice(text.as_bytes());
        self.used = end;
        Ok(())
    }
}

fn main() {
    let mut buffer = Buffer {
        bytes: [0; 128],
        used: 0,
    };
    // A caller's terminal/Unicode provider established that these two glyphs use
    // four display columns. ztui neither guesses this width nor slices UTF-8.
    Row::with_columns("界面", 4)
        .write_padded(&mut buffer, 12)
        .unwrap();
    println!(
        "{:?}",
        core::str::from_utf8(&buffer.bytes[..buffer.used]).unwrap()
    );
}
