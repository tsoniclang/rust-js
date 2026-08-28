use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::time::Instant;

use crate::value::JsValue;

thread_local! {
    static GLOBAL_CONSOLE: RefCell<Console> = RefCell::new(Console::new());
}

#[derive(Debug)]
pub struct Console {
    counts: BTreeMap<String, u64>,
    timers: BTreeMap<String, Instant>,
    profiles: BTreeMap<String, Instant>,
    group_depth: usize,
    group_indentation: usize,
    ignore_errors: bool,
    color_mode: ConsoleColorMode,
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl Console {
    pub fn new() -> Self {
        Self::with_options(ConsoleOptions::default())
    }

    pub fn with_options(options: ConsoleOptions) -> Self {
        Self {
            counts: BTreeMap::new(),
            timers: BTreeMap::new(),
            profiles: BTreeMap::new(),
            group_depth: 0,
            group_indentation: options.group_indentation,
            ignore_errors: options.ignore_errors,
            color_mode: options.color_mode,
        }
    }

    pub fn ignore_errors(&self) -> bool {
        self.ignore_errors
    }

    pub fn color_mode(&self) -> ConsoleColorMode {
        self.color_mode
    }

    pub fn group_indentation(&self) -> usize {
        self.group_indentation
    }

    pub fn format(&self, args: &[JsValue]) -> String {
        let prefix = " ".repeat(self.group_depth * self.group_indentation);
        if prefix.is_empty() {
            format_args(args)
        } else {
            format!("{prefix}{}", format_args(args))
        }
    }

    pub fn log_to(&self, writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
        writeln!(writer, "{}", self.format(args))
    }

    pub fn info_to(&self, writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
        self.log_to(writer, args)
    }

    pub fn debug_to(&self, writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
        self.log_to(writer, args)
    }

    pub fn warn_to(&self, writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
        writeln!(writer, "{}", self.format(args))
    }

    pub fn error_to(&self, writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
        writeln!(writer, "{}", self.format(args))
    }

    pub fn dir_to(&self, writer: &mut impl Write, item: &JsValue) -> io::Result<()> {
        writeln!(writer, "{}", item.inspect())
    }

    pub fn table_to(&self, writer: &mut impl Write, rows: &[JsValue]) -> io::Result<()> {
        writeln!(writer, "(index) Values")?;
        for (index, row) in rows.iter().enumerate() {
            writeln!(writer, "{index}: {}", row.inspect())?;
        }
        Ok(())
    }

    pub fn dirxml_to(&self, writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
        self.log_to(writer, args)
    }

    pub fn trace_to(&self, writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
        if args.is_empty() {
            writeln!(writer, "Trace")
        } else {
            writeln!(writer, "Trace: {}", self.format(args))
        }
    }

    pub fn assert_to(
        &self,
        writer: &mut impl Write,
        condition: bool,
        args: &[JsValue],
    ) -> io::Result<()> {
        if condition {
            Ok(())
        } else if args.is_empty() {
            writeln!(writer, "Assertion failed")
        } else {
            writeln!(writer, "Assertion failed: {}", self.format(args))
        }
    }

    pub fn count_to(&mut self, writer: &mut impl Write, label: Option<&str>) -> io::Result<u64> {
        let label = label.unwrap_or("default").to_string();
        let count = self.counts.entry(label.clone()).or_insert(0);
        *count += 1;
        writeln!(writer, "{label}: {count}")?;
        Ok(*count)
    }

    pub fn count_reset(&mut self, label: Option<&str>) {
        self.counts.remove(label.unwrap_or("default"));
    }

    pub fn time(&mut self, label: Option<&str>) {
        self.timers
            .insert(label.unwrap_or("default").to_string(), Instant::now());
    }

    pub fn time_log_to(
        &self,
        writer: &mut impl Write,
        label: Option<&str>,
        args: &[JsValue],
    ) -> io::Result<Option<u128>> {
        let label = label.unwrap_or("default");
        let Some(start) = self.timers.get(label) else {
            return Ok(None);
        };
        let elapsed = start.elapsed().as_millis();
        if args.is_empty() {
            writeln!(writer, "{label}: {elapsed}ms")?;
        } else {
            writeln!(writer, "{label}: {elapsed}ms {}", format_args(args))?;
        }
        Ok(Some(elapsed))
    }

    pub fn time_end_to(
        &mut self,
        writer: &mut impl Write,
        label: Option<&str>,
    ) -> io::Result<Option<u128>> {
        let label = label.unwrap_or("default");
        let Some(start) = self.timers.remove(label) else {
            return Ok(None);
        };
        let elapsed = start.elapsed().as_millis();
        writeln!(writer, "{label}: {elapsed}ms")?;
        Ok(Some(elapsed))
    }

    pub fn group_to(&mut self, writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
        if !args.is_empty() {
            self.log_to(writer, args)?;
        }
        self.group_depth += 1;
        Ok(())
    }

    pub fn group_collapsed_to(
        &mut self,
        writer: &mut impl Write,
        args: &[JsValue],
    ) -> io::Result<()> {
        self.group_to(writer, args)
    }

    pub fn group_end(&mut self) {
        self.group_depth = self.group_depth.saturating_sub(1);
    }

    pub fn clear_to(&self, writer: &mut impl Write) -> io::Result<()> {
        writer.write_all(b"\x1b[2J\x1b[H")
    }

    pub fn time_stamp_to(&self, writer: &mut impl Write, label: Option<&str>) -> io::Result<()> {
        if let Some(label) = label {
            writeln!(writer, "Timestamp: {label}")
        } else {
            writeln!(writer, "Timestamp")
        }
    }

    pub fn profile(&mut self, label: Option<&str>) {
        self.profiles
            .insert(label.unwrap_or("default").to_string(), Instant::now());
    }

    pub fn profile_end_to(
        &mut self,
        writer: &mut impl Write,
        label: Option<&str>,
    ) -> io::Result<Option<u128>> {
        let label = label.unwrap_or("default");
        let Some(start) = self.profiles.remove(label) else {
            return Ok(None);
        };
        let elapsed = start.elapsed().as_millis();
        writeln!(writer, "Profile '{label}': {elapsed}ms")?;
        Ok(Some(elapsed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsoleOptions {
    pub ignore_errors: bool,
    pub color_mode: ConsoleColorMode,
    pub group_indentation: usize,
}

impl Default for ConsoleOptions {
    fn default() -> Self {
        Self {
            ignore_errors: true,
            color_mode: ConsoleColorMode::Auto,
            group_indentation: 2,
        }
    }
}

pub fn log(args: &[JsValue]) {
    let mut out = io::stdout();
    let _ = log_to(&mut out, args);
}

pub fn error(args: &[JsValue]) {
    let mut out = io::stderr();
    let _ = error_to(&mut out, args);
}

pub fn warn(args: &[JsValue]) {
    let mut out = io::stderr();
    let _ = warn_to(&mut out, args);
}

pub fn info(args: &[JsValue]) {
    log(args);
}

pub fn debug(args: &[JsValue]) {
    log(args);
}

pub fn assert(condition: bool, args: &[JsValue]) {
    let mut out = io::stderr();
    GLOBAL_CONSOLE.with_borrow(|console| {
        let _ = console.assert_to(&mut out, condition, args);
    });
}

pub fn assert_default() {
    assert(false, &[]);
}

pub fn clear() {
    let mut out = io::stdout();
    GLOBAL_CONSOLE.with_borrow(|console| {
        let _ = console.clear_to(&mut out);
    });
}

pub fn count() {
    count_label("default");
}

pub fn count_label(label: &str) {
    let mut out = io::stdout();
    GLOBAL_CONSOLE.with_borrow_mut(|console| {
        let _ = console.count_to(&mut out, Some(label));
    });
}

pub fn count_reset() {
    count_reset_label("default");
}

pub fn count_reset_label(label: &str) {
    GLOBAL_CONSOLE.with_borrow_mut(|console| console.count_reset(Some(label)));
}

pub fn dir(item: &JsValue) {
    let mut out = io::stdout();
    let _ = dir_to(&mut out, item);
}

pub fn dir_with_options(item: &JsValue, _options: &JsValue) {
    dir(item);
}

pub fn dirxml(args: &[JsValue]) {
    let mut out = io::stdout();
    let _ = dirxml_to(&mut out, args);
}

pub fn group(args: &[JsValue]) {
    let mut out = io::stdout();
    GLOBAL_CONSOLE.with_borrow_mut(|console| {
        let _ = console.group_to(&mut out, args);
    });
}

pub fn group_collapsed(args: &[JsValue]) {
    let mut out = io::stdout();
    GLOBAL_CONSOLE.with_borrow_mut(|console| {
        let _ = console.group_collapsed_to(&mut out, args);
    });
}

pub fn group_end() {
    GLOBAL_CONSOLE.with_borrow_mut(Console::group_end);
}

pub fn table(rows: &[JsValue]) {
    let mut out = io::stdout();
    let _ = table_to(&mut out, rows);
}

pub fn time() {
    time_label("default");
}

pub fn time_label(label: &str) {
    GLOBAL_CONSOLE.with_borrow_mut(|console| console.time(Some(label)));
}

pub fn time_log(args: &[JsValue]) {
    time_log_label("default", args);
}

pub fn time_log_label(label: &str, args: &[JsValue]) {
    let mut out = io::stdout();
    GLOBAL_CONSOLE.with_borrow(|console| {
        let _ = console.time_log_to(&mut out, Some(label), args);
    });
}

pub fn time_end() {
    time_end_label("default");
}

pub fn time_end_label(label: &str) {
    let mut out = io::stdout();
    GLOBAL_CONSOLE.with_borrow_mut(|console| {
        let _ = console.time_end_to(&mut out, Some(label));
    });
}

pub fn time_stamp() {
    time_stamp_label("default");
}

pub fn time_stamp_label(label: &str) {
    let mut out = io::stdout();
    GLOBAL_CONSOLE.with_borrow(|console| {
        let _ = console.time_stamp_to(&mut out, Some(label));
    });
}

pub fn trace(args: &[JsValue]) {
    let mut out = io::stderr();
    let _ = trace_to(&mut out, args);
}

pub fn log_to(writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
    writeln!(writer, "{}", format_args(args))
}

pub fn error_to(writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
    writeln!(writer, "{}", format_args(args))
}

pub fn warn_to(writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
    writeln!(writer, "{}", format_args(args))
}

pub fn info_to(writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
    log_to(writer, args)
}

pub fn debug_to(writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
    log_to(writer, args)
}

pub fn dir_to(writer: &mut impl Write, item: &JsValue) -> io::Result<()> {
    writeln!(writer, "{}", item.inspect())
}

pub fn table_to(writer: &mut impl Write, rows: &[JsValue]) -> io::Result<()> {
    Console::new().table_to(writer, rows)
}

pub fn dirxml_to(writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
    Console::new().dirxml_to(writer, args)
}

pub fn trace_to(writer: &mut impl Write, args: &[JsValue]) -> io::Result<()> {
    Console::new().trace_to(writer, args)
}

pub fn format_args(args: &[JsValue]) -> String {
    args.iter()
        .map(|value| match value {
            JsValue::String(text) => text.to_utf8_lossy(),
            value => value.inspect(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}
