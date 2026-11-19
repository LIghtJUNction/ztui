//! ANSI row diffs over caller-owned frame data and output.
use crate::geometry::{Point, Size};
use core::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    pub position: Point,
    pub visible: bool,
}

/// Only modes owned by this renderer; session enter/leave restoration is external.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modes {
    pub application_cursor: bool,
    pub application_keypad: bool,
    pub bracketed_paste: bool,
    pub mouse_capture: bool,
}

/// Borrow a frame without creating another row vector or screen store.
pub trait Frame {
    fn size(&self) -> Size;

    /// Called only with row < size().rows. Return one already-laid-out row.
    ///
    /// Rows must exclude newline, cursor movement and non-SGR control sequences.
    /// Pad to the declared viewport width when a uniform background is required.
    /// Unicode width, clipping, trusted ANSI content, and grapheme segmentation
    /// are the caller's responsibility.
    fn row(&self, row: u16) -> &str;

    fn cursor(&self) -> Cursor;
    fn modes(&self) -> Modes;

    /// Override when pane geometry changes require a full redraw without resize.
    fn same_layout(&self, previous: &Self) -> bool {
        self.size() == previous.size()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiffStats {
    pub rows_written: u16,
    pub bytes_written: usize,
    pub full_redraw: bool,
}

struct Output<'a, W: fmt::Write + ?Sized> {
    inner: &'a mut W,
    bytes: usize,
}

impl<W: fmt::Write + ?Sized> fmt::Write for Output<'_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.inner.write_str(text)?;
        self.bytes = self.bytes.saturating_add(text.len());
        Ok(())
    }
}

fn begin<W: fmt::Write + ?Sized>(output: &mut W, started: &mut bool) -> fmt::Result {
    if !*started {
        output.write_str("\x1b[?25l")?;
        *started = true;
    }
    Ok(())
}

// Coordinates occupy at most five decimal digits. Fill the fixed ANSI
// templates backwards so one write forwards each complete control sequence.
#[inline]
fn decimal(buffer: &mut [u8], mut value: u32) -> usize {
    let mut start = buffer.len();
    loop {
        start -= 1;
        buffer[start] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            return start;
        }
    }
}

fn write_row<W: fmt::Write + ?Sized>(output: &mut W, row: u16, line: &str) -> fmt::Result {
    let mut command = *b"\x1b[00000;1H\x1b[0m\x1b[2K";
    let digits = decimal(&mut command[..7], u32::from(row) + 1);
    command[digits - 2] = 0x1b;
    command[digits - 1] = b'[';
    output.write_str(core::str::from_utf8(&command[digits - 2..]).unwrap())?;
    output.write_str(line)
}

fn write_cursor<W: fmt::Write + ?Sized>(
    output: &mut W,
    position: Point,
    visible: bool,
) -> fmt::Result {
    let mut command = *b"\x1b[0m\x1b[00000;00000H\x1b[?25h";
    let column = decimal(&mut command[..17], u32::from(position.column) + 1);
    command[column - 1] = b';';
    let row = decimal(&mut command[..column - 1], u32::from(position.row) + 1);
    let start = row - 6;
    command[start..row].copy_from_slice(b"\x1b[0m\x1b[");
    *command.last_mut().unwrap() = if visible { b'h' } else { b'l' };
    output.write_str(core::str::from_utf8(&command[start..]).unwrap())
}

/// Emit only changed rows, then owned mode changes and the final cursor.
///
/// An unchanged or empty frame writes zero bytes. The caller must pass the last
/// fully written frame as previous; after a short/failed terminal write use None
/// to redraw. This routine performs no allocation and no I/O, and uses O(1)
/// auxiliary memory. The sink may return fmt::Error after a partial write.
pub fn write_diff<W: fmt::Write + ?Sized, F: Frame>(
    output: &mut W,
    current: &F,
    previous: Option<&F>,
) -> Result<DiffStats, fmt::Error> {
    use core::fmt::Write as _;
    let size = current.size();
    if size.is_empty() {
        return Ok(DiffStats::default());
    }
    let full_redraw = previous.is_none_or(|old| old.size() != size || !current.same_layout(old));
    let mut output = Output {
        inner: output,
        bytes: 0,
    };
    let mut started = false;
    let mut rows_written = 0;
    for row in 0..size.rows {
        let line = current.row(row);
        if full_redraw
            || previous.is_none_or(|old| {
                let old_line = old.row(row);
                !core::ptr::eq(old_line, line) && old_line != line
            })
        {
            begin(&mut output, &mut started)?;
            write_row(&mut output, row, line)?;
            rows_written += 1;
        }
    }
    let modes = current.modes();
    let cursor = current.cursor();
    let modes_changed = previous.is_none_or(|old| old.modes() != modes);
    let cursor_changed = previous.is_none_or(|old| old.cursor() != cursor);
    if !started && !modes_changed && !cursor_changed {
        return Ok(DiffStats::default());
    }
    begin(&mut output, &mut started)?;
    let old_modes = previous.map(Frame::modes);
    if old_modes.is_none_or(|old| old.application_cursor != modes.application_cursor) {
        output.write_str(if modes.application_cursor {
            "\x1b[?1h"
        } else {
            "\x1b[?1l"
        })?;
    }
    if old_modes.is_none_or(|old| old.application_keypad != modes.application_keypad) {
        output.write_str(if modes.application_keypad {
            "\x1b="
        } else {
            "\x1b>"
        })?;
    }
    if old_modes.is_none_or(|old| old.mouse_capture != modes.mouse_capture) {
        output.write_str(if modes.mouse_capture {
            "\x1b[?1000h\x1b[?1006h"
        } else {
            "\x1b[?1000l\x1b[?1006l"
        })?;
    }
    if old_modes.is_none_or(|old| old.bracketed_paste != modes.bracketed_paste) {
        output.write_str(if modes.bracketed_paste {
            "\x1b[?2004h"
        } else {
            "\x1b[?2004l"
        })?;
    }
    // Nonempty geometry was checked before emitting any bytes.
    let position = size.clamp(cursor.position).unwrap();
    write_cursor(&mut output, position, cursor.visible)?;
    Ok(DiffStats {
        rows_written,
        bytes_written: output.bytes,
        full_redraw,
    })
}
