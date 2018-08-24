#[macro_use]
extern crate structopt;
extern crate failure;
extern crate romhack_backend;
extern crate termcolor;

mod opt;

use failure::{Error, ResultExt};
use opt::Opt;
use romhack_backend::{apply_patch, build, new, KeyValPrint, MessageKind};
use std::io::prelude::*;
use structopt::StructOpt;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

fn main() {
    if let Err(e) = try_main() {
        eprintln!();

        let mut bufwtr = BufferWriter::stderr(ColorChoice::Always);
        let mut buffer = bufwtr.buffer();
        let mut color = ColorSpec::new();
        color.set_fg(Some(Color::Red)).set_bold(true);

        buffer
            .set_color(&color)
            .expect("Error while printing error");
        write!(&mut buffer, "Error").expect("Error while printing error");

        buffer.reset().expect("Error while printing error");
        buffer
            .set_color(ColorSpec::new().set_bold(true))
            .expect("Error while printing error");
        writeln!(&mut buffer, ": {}", e).expect("Error while printing error");

        for cause in e.iter_chain().skip(1) {
            buffer
                .set_color(&color)
                .expect("Error while printing error");
            write!(&mut buffer, "   Caused by").expect("Error while printing error");

            buffer.reset().expect("Error while printing error");
            writeln!(&mut buffer, " {}", cause).expect("Error while printing error");
        }
        bufwtr.print(&buffer).expect("Error while printing error");
    } else {
        key_val_print(None, "Finished", "Rom Hack");
    }
}

fn try_main() -> Result<(), Error> {
    let opt = Opt::from_args();

    match opt {
        Opt::Build { debug, patch } => {
            build(&TermPrinter, debug, patch).context("Couldn't build the Rom Hack")?
        }
        Opt::New { name } => new(&name).context("Couldn't create the Rom Hack project")?,
        Opt::Apply {
            patch,
            original_game,
            output,
        } => apply_patch(&TermPrinter, patch, original_game, output)
            .context("Couldn't apply the patch")?,
    }

    Ok(())
}

pub struct TermPrinter;

impl KeyValPrint for TermPrinter {
    fn print(&self, kind: Option<MessageKind>, key: &str, val: &str) {
        let color = match kind {
            Some(MessageKind::Warning) => Some(Color::Yellow),
            Some(MessageKind::Error) => Some(Color::Red),
            None => None,
        };
        key_val_print(color, key, val)
    }
}

fn key_val_print(color: Option<Color>, key: &str, val: &str) {
    let bufwtr = BufferWriter::stderr(ColorChoice::Always);
    let mut buffer = bufwtr.buffer();

    buffer
        .set_color(
            ColorSpec::new()
                .set_fg(Some(color.unwrap_or(Color::Green)))
                .set_bold(true),
        ).ok();
    write!(&mut buffer, "{:>12}", key).ok();

    buffer.reset().ok();
    writeln!(&mut buffer, " {}", val).ok();
    bufwtr.print(&buffer).ok();
}
