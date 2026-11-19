use core::fmt;
use ztuio::{Cursor, Frame, Modes, Point, Rect, Row, RowError, Size, write_diff, write_spaces};

#[derive(Clone)]
struct Fixture {
    rows: Vec<String>,
    size: Size,
    cursor: Cursor,
    modes: Modes,
    layout: u8,
}

impl Fixture {
    fn new(lines: &[&str], columns: u16) -> Self {
        Self {
            rows: lines.iter().map(|line| (*line).to_owned()).collect(),
            size: Size {
                rows: lines.len().try_into().unwrap(),
                columns,
            },
            cursor: Cursor::default(),
            modes: Modes::default(),
            layout: 0,
        }
    }
}

impl Frame for Fixture {
    fn size(&self) -> Size {
        self.size
    }
    fn row(&self, row: u16) -> &str {
        &self.rows[usize::from(row)]
    }
    fn cursor(&self) -> Cursor {
        self.cursor
    }
    fn modes(&self) -> Modes {
        self.modes
    }
    fn same_layout(&self, previous: &Self) -> bool {
        self.size == previous.size && self.layout == previous.layout
    }
}

#[test]
fn first_frame_has_exact_ansi_mode_and_cursor_sequence() {
    let frame = Fixture::new(&["abc"], 3);
    let mut out = String::new();
    let stats = write_diff(&mut out, &frame, None).unwrap();
    assert_eq!(
        out,
        "\x1b[?25l\x1b[1;1H\x1b[0m\x1b[2Kabc\
         \x1b[?1l\x1b>\x1b[?1000l\x1b[?1006l\x1b[?2004l\
         \x1b[0m\x1b[1;1H\x1b[?25l"
    );
    assert_eq!(stats.rows_written, 1);
    assert_eq!(stats.bytes_written, out.len());
    assert!(stats.full_redraw);
}

#[test]
fn unchanged_and_empty_frames_do_not_touch_caller_output() {
    let frame = Fixture::new(&["same"], 4);
    let mut out = String::from("already written");
    let stats = write_diff(&mut out, &frame, Some(&frame)).unwrap();
    assert_eq!(stats.bytes_written, 0);
    assert_eq!(stats.rows_written, 0);
    assert_eq!(out, "already written");
    let empty = Fixture::new(&[], 0);
    assert_eq!(write_diff(&mut out, &empty, None).unwrap().bytes_written, 0);
    assert_eq!(out, "already written");
}

#[test]
fn one_row_change_uses_absolute_position_without_repainting_other_rows() {
    let before = Fixture::new(&["first", "old  ", "last "], 5);
    let mut after = before.clone();
    after.rows[1] = "new  ".into();
    let mut out = String::new();
    let stats = write_diff(&mut out, &after, Some(&before)).unwrap();
    assert_eq!(stats.rows_written, 1);
    assert!(!stats.full_redraw);
    assert!(out.contains("\x1b[2;1H\x1b[0m\x1b[2Knew  "));
    assert!(!out.contains("\x1b[1;1H\x1b[0m\x1b[2K"));
    assert!(!out.contains("\x1b[3;1H"));
    assert!(!out.contains("\x1b[?2004"));
}

#[test]
fn geometry_change_and_layout_key_force_a_full_row_redraw() {
    let before = Fixture::new(&["first", "last "], 5);
    for after in [
        Fixture::new(&["first ", "last  ", "new   "], 6),
        Fixture {
            layout: 1,
            ..before.clone()
        },
    ] {
        let mut out = String::new();
        let stats = write_diff(&mut out, &after, Some(&before)).unwrap();
        assert!(stats.full_redraw);
        assert_eq!(stats.rows_written, after.size.rows);
        assert_eq!(stats.bytes_written, out.len());
    }
}

#[test]
fn cursor_and_mode_only_changes_do_not_repaint_rows() {
    let before = Fixture::new(&["content "], 8);
    let mut after = before.clone();
    after.cursor = Cursor {
        position: Point {
            row: 50,
            column: 50,
        },
        visible: true,
    };
    after.modes.mouse_capture = true;
    let mut out = String::new();
    let stats = write_diff(&mut out, &after, Some(&before)).unwrap();
    assert_eq!(stats.rows_written, 0);
    assert_eq!(
        out,
        "\x1b[?25l\x1b[?1000h\x1b[?1006h\x1b[0m\x1b[1;8H\x1b[?25h"
    );
}

#[test]
fn unicode_and_sgr_rows_are_forwarded_without_hidden_width_guessing() {
    let line = "\x1b[38;2;20;40;60m中文界\x1b[0m  ";
    let frame = Fixture::new(&[line], 8);
    let mut out = String::new();
    let stats = write_diff(&mut out, &frame, None).unwrap();
    assert_eq!(
        out,
        format!(
            "\x1b[?25l\x1b[1;1H\x1b[0m\x1b[2K{line}\
             \x1b[?1l\x1b>\x1b[?1000l\x1b[?1006l\x1b[?2004l\
             \x1b[0m\x1b[1;1H\x1b[?25l"
        )
    );
    assert_eq!(stats.bytes_written, out.len());
    let mut calls = 0;
    let measured = Row::measured("中文界", |_| {
        calls += 1;
        6
    });
    assert_eq!(calls, 1);
    let mut row = String::new();
    measured.write_padded(&mut row, 8).unwrap();
    assert_eq!(row, "中文界  ");
}

#[test]
fn width_rejection_happens_before_any_output_and_keeps_utf8_intact() {
    let mut out = String::from("prefix");
    let error = Row::with_columns("界", 2)
        .write_padded(&mut out, 1)
        .unwrap_err();
    assert_eq!(
        error,
        RowError::TooWide {
            columns: 2,
            available: 1
        }
    );
    assert_eq!(out, "prefix");
    Row::with_columns("", 0).write_padded(&mut out, 0).unwrap();
    assert_eq!(out, "prefix");
    Row::with_columns("e\u{301}", 1)
        .write_padded(&mut out, 2)
        .unwrap();
    assert_eq!(out, "prefixe\u{301} ");
}

#[test]
fn padding_crosses_chunk_boundaries_without_losing_spaces() {
    for width in [0, 1, 63, 64, 65, 128, 255] {
        let mut out = String::new();
        write_spaces(&mut out, width).unwrap();
        assert_eq!(out.len(), width);
        assert!(out.bytes().all(|byte| byte == b' '));
    }
}

struct Limited {
    bytes: [u8; 24],
    used: usize,
}
impl fmt::Write for Limited {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.len() > self.bytes.len() - self.used {
            return Err(fmt::Error);
        }
        self.bytes[self.used..self.used + text.len()].copy_from_slice(text.as_bytes());
        self.used += text.len();
        Ok(())
    }
}

#[test]
fn caller_fixed_buffers_receive_output_errors_without_allocation_or_panics() {
    let mut out = Limited {
        bytes: [0; 24],
        used: 0,
    };
    Row::with_columns("hello", 5)
        .write_padded(&mut out, 8)
        .unwrap();
    assert_eq!(&out.bytes[..out.used], b"hello   ");
    let frame = Fixture::new(&["more than the remaining capacity"], 32);
    assert!(write_diff(&mut out, &frame, None).is_err());
    assert!(out.used <= out.bytes.len());
}

#[test]
fn rectangles_reject_overflow_and_clip_hidden_panes() {
    assert!(
        Rect::new(
            Point {
                row: u16::MAX,
                column: 0
            },
            Size {
                rows: 1,
                columns: 1
            }
        )
        .is_none()
    );
    assert!(
        Rect::new(
            Point { row: 0, column: 1 },
            Size {
                rows: 1,
                columns: u16::MAX
            }
        )
        .is_none()
    );
    let rect = Rect::new(
        Point { row: 8, column: 9 },
        Size {
            rows: 8,
            columns: 8,
        },
    )
    .unwrap();
    let clipped = rect.clipped(Size {
        rows: 10,
        columns: 12,
    });
    assert_eq!(
        clipped.size(),
        Size {
            rows: 2,
            columns: 3
        }
    );
    assert!(clipped.contains(Point { row: 9, column: 11 }));
    assert!(!clipped.contains(Point {
        row: 10,
        column: 12
    }));
    assert!(
        rect.clipped(Size {
            rows: 2,
            columns: 3
        })
        .size()
        .is_empty()
    );
}

#[test]
fn pane_splits_conserve_cells_and_never_overlap() {
    for width in 0..65 {
        let rect = Rect::new(
            Point { row: 2, column: 3 },
            Size {
                rows: 10,
                columns: width,
            },
        )
        .unwrap();
        for wanted in [0, 1, 20, u16::MAX] {
            let (left, right) = rect.split_columns(wanted, 1);
            assert!(left.right() <= right.origin().column);
            assert_eq!(right.right(), rect.right());
            assert_eq!(
                left.size().columns + right.size().columns + right.origin().column - left.right(),
                width
            );
            let (top, bottom) = rect.split_rows(wanted, 1);
            assert!(top.bottom() <= bottom.origin().row);
            assert_eq!(bottom.bottom(), rect.bottom());
        }
    }
}

#[test]
fn equal_rows_in_distinct_allocations_emit_nothing() {
    let before = Fixture::new(&["\x1b[31m中文 e\u{301}\x1b[0m", "same"], 16);
    let after = before.clone();
    assert_ne!(before.rows[0].as_ptr(), after.rows[0].as_ptr());
    let mut out = String::from("existing output");
    let stats = write_diff(&mut out, &after, Some(&before)).unwrap();
    assert_eq!(stats, ztuio::DiffStats::default());
    assert_eq!(out, "existing output");
}

#[test]
fn all_coordinate_digit_boundaries_match_the_ansi_reference() {
    use core::fmt::Write as _;

    struct LargeFrame {
        cursor: Cursor,
    }
    impl Frame for LargeFrame {
        fn size(&self) -> Size {
            Size {
                rows: u16::MAX,
                columns: u16::MAX,
            }
        }
        fn row(&self, _: u16) -> &str {
            ""
        }
        fn cursor(&self) -> Cursor {
            self.cursor
        }
        fn modes(&self) -> Modes {
            Modes::default()
        }
    }

    let mut expected = String::from("\x1b[?25l");
    for row in 1..=u16::MAX {
        write!(expected, "\x1b[{row};1H\x1b[0m\x1b[2K").unwrap();
    }
    expected.push_str(
        "\x1b[?1l\x1b>\x1b[?1000l\x1b[?1006l\x1b[?2004l\
         \x1b[0m\x1b[65535;65535H\x1b[?25h",
    );
    let mut out = String::new();
    let frame = LargeFrame {
        cursor: Cursor {
            position: Point {
                row: u16::MAX,
                column: u16::MAX,
            },
            visible: true,
        },
    };
    let stats = write_diff(&mut out, &frame, None).unwrap();
    assert!(
        out == expected,
        "coordinate bytes differ from the ANSI reference"
    );
    assert_eq!(stats.rows_written, u16::MAX);
    assert_eq!(stats.bytes_written, expected.len());
    assert!(stats.full_redraw);

    let boundaries = [
        0,
        8,
        9,
        98,
        99,
        998,
        999,
        9998,
        9999,
        u16::MAX - 1,
        u16::MAX,
    ];
    for row in boundaries {
        for column in boundaries {
            for visible in [false, true] {
                let before = LargeFrame {
                    cursor: Cursor {
                        visible: !visible,
                        ..Cursor::default()
                    },
                };
                let after = LargeFrame {
                    cursor: Cursor {
                        position: Point { row, column },
                        visible,
                    },
                };
                out.clear();
                let stats = write_diff(&mut out, &after, Some(&before)).unwrap();
                let expected = format!(
                    "\x1b[?25l\x1b[0m\x1b[{};{}H\x1b[?25{}",
                    u32::from(row.min(u16::MAX - 1)) + 1,
                    u32::from(column.min(u16::MAX - 1)) + 1,
                    if visible { 'h' } else { 'l' },
                );
                assert_eq!(out, expected);
                assert_eq!(stats.rows_written, 0);
                assert_eq!(stats.bytes_written, expected.len());
                assert!(!stats.full_redraw);
            }
        }
    }
}
