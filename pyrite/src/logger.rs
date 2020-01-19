use log::{Level, Metadata, Record};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
// use std::sync::atomic::AtomicUsize;

// // #TODO there's probably a better way to do this.
// /// Compares the current value of an atomic integer with `new_value` and writes the `new_value` if it is
// /// larger. This will return the maximum of the two.
// fn atomic_set_max(atomic: &AtomicUsize, new_value: usize) -> usize {
//     use std::sync::atomic::Ordering;
//     let mut read_value = atomic.load(Ordering::Acquire);
//     loop {
//         if new_value <= read_value {
//             return read_value;
//         }
//         read_value = atomic.compare_and_swap(read_value, new_value, Ordering::AcqRel);
//     }
// }

pub struct PyriteLogger {
    output: StandardStream,
    max_level: Level,
    // max_target_len: AtomicUsize,
}

impl PyriteLogger {
    pub fn init(max_level: Level) {
        log::set_boxed_logger(Box::new(PyriteLogger {
            max_level,
            output: StandardStream::stdout(ColorChoice::Auto),
            // max_target_len: AtomicUsize::new(0),
        }))
        .map(|()| log::set_max_level(max_level.to_level_filter()))
        .expect("failed to initialize pyrite logger");
    }

    fn text_for_level(level: Level) -> &'static str {
        match level {
            Level::Error => "ERROR",
            Level::Warn => " WARN",
            Level::Info => " INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }

    fn color_for_level(level: Level) -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_bold(true);

        let fg_color = match level {
            Level::Error => Color::Red,
            Level::Warn => Color::Yellow,
            Level::Info => Color::Blue,
            Level::Debug => Color::Green,
            Level::Trace => Color::Magenta,
        };

        spec.set_fg(Some(fg_color));
        spec.set_bg(None);

        return spec;
    }

    fn color_for_body() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_bold(true);
        spec.set_fg(None);
        spec.set_bg(None);
        return spec;
    }

    fn color_for_target() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(None);
        spec.set_bg(None);
        return spec;
    }
}

impl log::Log for PyriteLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let mut out = self.output.lock();

        // Level:
        out.set_color(&PyriteLogger::color_for_level(record.level()))
            .expect("failed to set level color");
        write!(
            &mut out,
            "{} ",
            PyriteLogger::text_for_level(record.level())
        )
        .expect("failed to write log level");

        // Target:
        out.set_color(&PyriteLogger::color_for_target())
            .expect("failed to set message target color");
        // let max_target_len = atomic_set_max(&self.max_target_len, record.target().len());
        // write!(
        //     &mut out,
        //     "[{:>width$}] ",
        //     record.target(),
        //     width = record.target().len()
        // )
        // .expect("failed to write message body");
        write!(&mut out, "[{}] ", record.target(),).expect("failed to write message body");

        // Text:
        out.set_color(&PyriteLogger::color_for_body())
            .expect("failed to set message body color");
        write!(&mut out, "{}\n", record.args()).expect("failed to write message body");

        // Reset:
        out.set_color(ColorSpec::new().set_fg(None).set_bg(None))
            .expect("failed to reset color");
    }

    fn flush(&self) {}
}
