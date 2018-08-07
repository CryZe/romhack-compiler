extern crate byteorder;
extern crate encoding_rs;
#[macro_use]
extern crate failure;
extern crate goblin;
extern crate image;
extern crate regex;
extern crate rustc_demangle;
#[macro_use]
extern crate serde_derive;
extern crate serde;
#[macro_use]
extern crate structopt;
extern crate standalone_syn as syn;
extern crate termcolor;
extern crate toml;

mod assembler;
mod banner;
mod config;
mod demangle;
mod dol;
mod framework_map;
mod iso;
mod linker;
mod opt;

use assembler::Assembler;
use assembler::Instruction;
use banner::Banner;
use config::Config;
use dol::DolFile;
use failure::{err_msg, Error, ResultExt};
use opt::Opt;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{prelude::*, BufWriter};
use std::process::Command;
use structopt::StructOpt;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

fn build() -> Result<(), Error> {
    let mut toml_buf = String::new();
    File::open("RomHack.toml")
        .context("Couldn't find \"RomHack.toml\".")?
        .read_to_string(&mut toml_buf)
        .context("Failed to read \"RomHack.toml\".")?;

    let mut config: Config = toml::from_str(&toml_buf).context("Can't parse RomHack.toml")?;
    let base_address: syn::LitInt =
        syn::parse_str(&config.link.base).context("Invalid Base Address")?;

    key_val_print(None, "Compiling", "");

    {
        let mut command = Command::new("cargo");
        command
            .args(&[
                "build",
                "--release",
                "--target",
                "powerpc-unknown-linux-gnu",
            ])
            .env(
                "RUSTFLAGS",
                "-C target-feature=+msync,+fres,+frsqrte -C opt-level=s",
            );
        if let Some(ref src_dir) = config.src.src {
            command.current_dir(src_dir);
        }

        let exit_code = command
            .spawn()
            .context("Couldn't build the project")?
            .wait()?;

        ensure!(exit_code.success(), "Couldn't build the project");
    }

    key_val_print(None, "Loading", "original game");

    let buf = iso::reader::load_iso_buf(&config.src.iso)
        .with_context(|_| format!("Couldn't find \"{}\".", config.src.iso.display()))?;

    let mut iso = iso::reader::load_iso(&buf).context("Couldn't parse the ISO")?;

    key_val_print(None, "Replacing", "files");

    for (iso_path, actual_path) in &config.files {
        iso.resolve_and_create_path(iso_path).data = fs::read(actual_path)
            .with_context(|_| {
                format!(
                    "Couldn't read the file \"{}\" to store it in the ISO.",
                    actual_path.display()
                )
            })?
            .into();
    }

    let mut original_symbols = HashMap::new();
    if let Some(framework_map) = config.src.map.as_ref().and_then(|m| iso.resolve_path(m)) {
        key_val_print(None, "Parsing", "symbol map");
        original_symbols = framework_map::parse(&framework_map.data)
            .context("Couldn't parse the game's symbol map")?;
    } else {
        key_val_print(
            Some(Color::Yellow),
            "Warning",
            "No symbol map specified or it wasn't found",
        );
    }

    key_val_print(None, "Linking", "");

    let mut libs_to_link = Vec::with_capacity(config.src.link.len() + 1);
    for lib_path in &config.src.link {
        let mut file_buf = fs::read(&config.src.link[0]).with_context(|_| {
            format!(
                "Couldn't load \"{}\". Did you build the project correctly?",
                lib_path.display()
            )
        })?;
        libs_to_link.push(file_buf);
    }
    libs_to_link.push(linker::BASIC_LIB.to_owned());

    let linked = linker::link(
        &libs_to_link,
        base_address.value() as u32,
        config.link.entries.clone(),
        &original_symbols,
    ).context("Couldn't link the Rom Hack")?;

    key_val_print(None, "Creating", "symbol map");

    // TODO NLL bind framework_map to local variable
    framework_map::create(
        &config,
        config
            .src
            .map
            .as_ref()
            .and_then(|m| iso.resolve_path(m))
            .map(|f| &*f.data),
        &linked.sections,
    ).context("    Couldn't create the new symbol map")?;

    let mut instructions = Vec::new();
    if let Some(patch) = config.src.patch.take() {
        key_val_print(None, "Parsing", "patch");

        let mut asm = String::new();
        File::open(&patch)
            .with_context(|_| format!("Couldn't find \"{}\".", patch.display()))?
            .read_to_string(&mut asm)
            .context("Couldn't read the patch file")?;

        let lines = &asm.lines().collect::<Vec<_>>();

        let mut assembler = Assembler::new(linked.symbol_table);
        instructions = assembler
            .assemble_all_lines(lines)
            .context("Couldn't assemble the patch file lines")?;
    }

    {
        key_val_print(None, "Patching", "game");

        let main_dol = iso
            .main_dol_mut()
            .ok_or_else(|| err_msg("Dol file not found"))?;

        let original = DolFile::parse(&main_dol.data);
        main_dol.data = patch_game(original, linked.dol, &instructions)
            .context("Couldn't patch the game")?
            .into();
    }
    {
        key_val_print(None, "Patching", "banner");

        if let Some(banner_file) = iso.banner_mut() {
            // TODO Not always true
            let is_japanese = true;
            let mut banner =
                Banner::parse(is_japanese, &banner_file.data).context("Couldn't parse the banner")?;

            if let Some(game_name) = config.info.game_name.take() {
                banner.game_name = game_name;
            }
            if let Some(developer_name) = config.info.developer_name.take() {
                banner.developer_name = developer_name;
            }
            if let Some(full_game_name) = config.info.full_game_name.take() {
                banner.full_game_name = full_game_name;
            }
            if let Some(full_developer_name) = config.info.full_developer_name.take() {
                banner.full_developer_name = full_developer_name;
            }
            if let Some(game_description) = config.info.description.take() {
                banner.game_description = game_description;
            }
            if let Some(image_path) = config.info.image.take() {
                let image = image::open(image_path)
                    .context("Couldn't open the banner replacement image")?
                    .to_rgba();
                banner.image.copy_from_slice(&image);
            }
            banner_file.data = banner.to_bytes(is_japanese).to_vec().into();
        } else {
            key_val_print(Some(Color::Yellow), "Warning", "No banner to patch");
        }
    }

    key_val_print(None, "Building", "ISO");
    let iso_path = &config.build.iso;
    iso::writer::write_iso(
        BufWriter::with_capacity(
            4 << 20,
            File::create(iso_path).context("Couldn't create the final ISO")?,
        ),
        &iso,
    ).context("Couldn't write the final ISO")?;

    Ok(())
}

fn new(name: String) -> Result<(), Error> {
    let exit_code = Command::new("cargo")
        .args(&["new", "--lib", &name])
        .spawn()
        .context("Couldn't create the cargo project")?
        .wait()?;

    ensure!(exit_code.success(), "Couldn't create the cargo project");

    let mut file =
        File::create(format!("{}/RomHack.toml", name)).context("Couldn't create the RomHack.toml")?;
    write!(
        file,
        r#"[info]
game-name = "{0}"

[src]
iso = "game.iso" # Provide the path of the game's ISO
link = ["target/powerpc-unknown-linux-gnu/release/lib{1}.a"]
patch = "src/patch.asm"
# Optionally specify the game's symbol map
# map = "maps/framework.map"

[files]
# You may replace or add new files to the game here
# "path/to/file/in/iso" = "path/to/file/on/harddrive"

[build]
map = "target/framework.map"
iso = "target/{0}.iso"

[link]
entries = ["init"] # Enter the exported function names here
base = "0x8040_1000" # Enter the start address of the Rom Hack's code here
"#,
        name,
        name.replace('-', "_"),
    ).context("Couldn't write the RomHack.toml")?;

    let mut file = File::create(format!("{}/src/lib.rs", name))
        .context("Couldn't create the lib.rs source file")?;
    write!(
        file,
        "{}",
        r#"#![no_std]
#![feature(panic_implementation)]
pub mod panic_impl;

#[no_mangle]
pub extern "C" fn init() {}
"#
    ).context("Couldn't write the lib.rs source file")?;

    let mut file = File::create(format!("{}/src/panic_impl.rs", name))
        .context("Couldn't create the panic_impl.rs source file")?;
    write!(
        file,
        "{}",
        r#"#[cfg(any(target_arch = "powerpc", target_arch = "wasm32"))]
#[panic_implementation]
#[no_mangle]
pub fn panic(_info: &::core::panic::PanicInfo) -> ! {
    loop {}
}
"#
    ).context("Couldn't write the lang_items.rs source file")?;

    let mut file = File::create(format!("{}/src/patch.asm", name))
        .context("Couldn't create the default patch file")?;
    write!(
        file,
        r#"; You can use this to patch the game's code to call into the Rom Hack's code
"#
    ).context("Couldn't write the default patch file")?;

    let mut file = OpenOptions::new()
        .append(true)
        .open(format!("{}/Cargo.toml", name))
        .context("Couldn't open the Cargo.toml")?;
    write!(
        file,
        r#"
[lib]
crate-type = ["staticlib"]

[profile.release]
panic = "abort"
lto = true
"#
    ).context("Couldn't write into the Cargo.toml")?;

    Ok(())
}

fn try_main() -> Result<(), Error> {
    let opt = Opt::from_args();

    match opt {
        Opt::Build {} => build().context("Couldn't build the Rom Hack")?,
        Opt::New { name } => new(name).context("Couldn't create the Rom Hack project")?,
    }

    Ok(())
}

fn key_val_print(color: Option<Color>, key: &str, val: &str) {
    let bufwtr = BufferWriter::stderr(ColorChoice::Always);
    let mut buffer = bufwtr.buffer();

    buffer
        .set_color(
            ColorSpec::new()
                .set_fg(Some(color.unwrap_or(Color::Green)))
                .set_bold(true),
        )
        .ok();
    write!(&mut buffer, "{:>12}", key).ok();

    buffer.reset().ok();
    writeln!(&mut buffer, " {}", val).ok();
    bufwtr.print(&buffer).ok();
}

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

fn patch_game(
    mut original: DolFile,
    intermediate: DolFile,
    instructions: &[Instruction],
) -> Result<Vec<u8>, Error> {
    original.append(intermediate);
    original
        .patch(instructions)
        .context("Couldn't patch the DOL")?;

    Ok(original.to_bytes())
}
