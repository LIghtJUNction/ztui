//! Fixed-width row contracts without an embedded Unicode-width table.
use core::fmt;

/// Already-rendered text and its caller-measured display-column count.
///
/// The text must be a single line. For ANSI text the caller must measure visible
/// cells, ignore SGR bytes, and exclude cursor movement/OSC/other controls. This
/// type deliberately does not sanitize terminal input or segment graphemes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row<'a> {
    text: &'a str,
    columns: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowError {
    TooWide { columns: usize, available: u16 },
    Output(fmt::Error),
}

impl fmt::Display for RowError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooWide { columns, available } => write!(
                output,
                "row uses {columns} columns but only {available} are available"
            ),
            Self::Output(error) => fmt::Display::fmt(error, output),
        }
    }
}

impl core::error::Error for RowError {}

impl<'a> Row<'a> {
    pub const fn with_columns(text: &'a str, columns: usize) -> Self {
        Self { text, columns }
    }

    /// The provider may use a full Unicode/terminal cell model outside ztuio.
    pub fn measured(text: &'a str, provider: impl FnOnce(&str) -> usize) -> Self {
        Self::with_columns(text, provider(text))
    }

    pub const fn text(self) -> &'a str {
        self.text
    }
    pub const fn columns(self) -> usize {
        self.columns
    }

    /// Write the row and enough ASCII spaces to fill the requested cell width.
    ///
    /// A width error is detected before writing anything. Output errors may leave
    /// partial bytes, as with fmt::Write. Padding uses the currently active style;
    /// set the intended background before calling. No Unicode text is truncated.
    pub fn write_padded<W: fmt::Write + ?Sized>(
        self,
        output: &mut W,
        width: u16,
    ) -> Result<(), RowError> {
        if self.columns > usize::from(width) {
            return Err(RowError::TooWide {
                columns: self.columns,
                available: width,
            });
        }
        output.write_str(self.text).map_err(RowError::Output)?;
        write_spaces(output, usize::from(width) - self.columns).map_err(RowError::Output)
    }
}

/// Write padding in fixed chunks; never allocate a temporary repeated string.
pub fn write_spaces<W: fmt::Write + ?Sized>(output: &mut W, mut columns: usize) -> fmt::Result {
    const SPACES: &str = "                                                                ";
    while columns != 0 {
        let count = columns.min(SPACES.len());
        output.write_str(&SPACES[..count])?;
        columns -= count;
    }
    Ok(())
}
