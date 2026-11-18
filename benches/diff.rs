//! Dependency-free old/new comparison. No terminal or model is involved.
use std::{
    fmt::Write as _,
    hint::black_box,
    time::{Duration, Instant},
};
use ztui::{Cursor, Frame, Modes, Point, Size};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Fixture {
    size: Size,
    rows: Vec<String>,
    cursor: Cursor,
    modes: Modes,
    layout: u64,
}
impl Fixture {
    fn new(rows: u16, columns: u16, marker: &str) -> Self {
        let rows_text = (0..rows)
            .map(|index| {
                let text = format!("row {index:03} {marker}");
                format!(
                    "\x1b[38;2;80;100;120m{text:<width$}\x1b[0m",
                    width = usize::from(columns)
                )
            })
            .collect();
        Self {
            size: Size { rows, columns },
            rows: rows_text,
            cursor: Cursor {
                position: Point {
                    row: rows - 1,
                    column: 2,
                },
                visible: true,
            },
            modes: Modes {
                bracketed_paste: true,
                mouse_capture: true,
                ..Modes::default()
            },
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
    fn same_layout(&self, old: &Self) -> bool {
        self.size == old.size && self.layout == old.layout
    }
}

/// The SSMCP 97c026c Frame::diff algorithm, with allocation moved to the caller
/// equally for both paths. It intentionally does not call the new implementation.
fn legacy(out: &mut String, current: &Fixture, previous: Option<&Fixture>) {
    if previous == Some(current) {
        return;
    }
    out.push_str("\x1b[?25l");
    let resized =
        previous.is_none_or(|old| current.size != old.size || current.layout != old.layout);
    for (row, line) in current.rows.iter().enumerate() {
        if resized || previous.and_then(|old| old.rows.get(row)) != Some(line) {
            write!(out, "\x1b[{};1H\x1b[0m\x1b[2K{line}", row + 1).unwrap();
        }
    }
    if previous.is_none_or(|old| old.modes.application_cursor != current.modes.application_cursor) {
        out.push_str(if current.modes.application_cursor {
            "\x1b[?1h"
        } else {
            "\x1b[?1l"
        });
    }
    if previous.is_none_or(|old| old.modes.application_keypad != current.modes.application_keypad) {
        out.push_str(if current.modes.application_keypad {
            "\x1b="
        } else {
            "\x1b>"
        });
    }
    if previous.is_none_or(|old| old.modes.mouse_capture != current.modes.mouse_capture) {
        out.push_str(if current.modes.mouse_capture {
            "\x1b[?1000h\x1b[?1006h"
        } else {
            "\x1b[?1000l\x1b[?1006l"
        });
    }
    if previous.is_none_or(|old| old.modes.bracketed_paste != current.modes.bracketed_paste) {
        out.push_str(if current.modes.bracketed_paste {
            "\x1b[?2004h"
        } else {
            "\x1b[?2004l"
        });
    }
    write!(
        out,
        "\x1b[0m\x1b[{};{}H\x1b[?25{}",
        current.cursor.position.row + 1,
        current.cursor.position.column + 1,
        if current.cursor.visible { 'h' } else { 'l' },
    )
    .unwrap();
}

fn modern(out: &mut String, current: &Fixture, previous: Option<&Fixture>) {
    ztui::write_diff(out, current, previous).unwrap();
}

type Renderer = fn(&mut String, &Fixture, Option<&Fixture>);
fn measure(
    renderer: Renderer,
    current: &Fixture,
    previous: Option<&Fixture>,
    iterations: u64,
) -> Duration {
    let mut out = String::with_capacity(
        usize::from(current.size.rows) * (usize::from(current.size.columns) + 64) + 128,
    );
    for _ in 0..200 {
        out.clear();
        renderer(&mut out, black_box(current), black_box(previous));
        black_box(out.as_bytes());
    }
    let started = Instant::now();
    for _ in 0..iterations {
        out.clear();
        renderer(&mut out, black_box(current), black_box(previous));
        black_box(out.as_bytes());
    }
    started.elapsed()
}

fn main() {
    let iterations = std::env::args()
        .skip(1)
        .find(|argument| argument != "--bench")
        .map(|s| s.parse::<u64>().expect("iterations must be an integer"))
        .unwrap_or(50_000)
        .clamp(1, 10_000_000);
    let before = Fixture::new(64, 160, "unchanged");
    let mut one = before.clone();
    one.rows[31] = Fixture::new(64, 160, "edited").rows.remove(31);
    let all = Fixture::new(64, 160, "changed");
    let resized = Fixture::new(64, 180, "resized");
    let mut cursor = before.clone();
    cursor.cursor.position.column += 1;
    let mut mode = before.clone();
    mode.modes.application_cursor = true;
    let cases = [
        ("first", &before, None),
        ("unchanged", &before, Some(&before)),
        ("one_row", &one, Some(&before)),
        ("all_rows", &all, Some(&before)),
        ("resize", &resized, Some(&before)),
        ("cursor_only", &cursor, Some(&before)),
        ("mode_only", &mode, Some(&before)),
    ];
    println!(
        "{{\"benchmark\":\"ztui-diff\",\"version\":\"{}\",\"architecture\":\"{}\",\"os\":\"{}\",\"iterations\":{},\"buffer\":\"caller-owned String reused for both paths\",\"includes_terminal_io\":false}}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        std::env::consts::OS,
        iterations
    );
    for (name, current, previous) in cases {
        let mut old = String::new();
        let mut new = String::new();
        legacy(&mut old, current, previous);
        let stats = ztui::write_diff(&mut new, current, previous).unwrap();
        assert_eq!(old, new, "ANSI bytes differ for {name}");
        assert_eq!(stats.bytes_written, new.len());
        // Reverse the order in the second round to reduce warm-up/order bias.
        let old_time = measure(legacy, current, previous, iterations);
        let new_time = measure(modern, current, previous, iterations);
        let new_time = new_time + measure(modern, current, previous, iterations);
        let old_time = old_time + measure(legacy, current, previous, iterations);
        let old_ns = old_time.as_secs_f64() * 1e9 / (iterations * 2) as f64;
        let new_ns = new_time.as_secs_f64() * 1e9 / (iterations * 2) as f64;
        println!(
            "{{\"scenario\":\"{name}\",\"rows\":{},\"columns\":{},\"bytes\":{},\"rows_written\":{},\"byte_equivalent\":true,\"legacy_ns_per_frame\":{old_ns:.2},\"ztui_ns_per_frame\":{new_ns:.2},\"ztui_over_legacy\":{:.4}}}",
            current.size.rows,
            current.size.columns,
            new.len(),
            stats.rows_written,
            new_ns / old_ns
        );
    }
}
