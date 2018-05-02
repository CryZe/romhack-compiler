extern crate byteorder;
extern crate encoding_rs;
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
use opt::Opt;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{prelude::*, BufWriter};
use std::process::Command;
use structopt::StructOpt;

fn build() {
    let mut toml_buf = String::new();
    File::open("RomHack.toml")
        .expect("Couldn't find \"RomHack.toml\".")
        .read_to_string(&mut toml_buf)
        .expect("Failed to read \"RomHack.toml\".");

    let mut config: Config = toml::from_str(&toml_buf).expect("Can't parse RomHack.toml");
    let base_address: syn::LitInt =
        syn::parse_str(&config.link.base).expect("Invalid Base Address");

    eprintln!("Compiling...");

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
            .expect("Couldn't build the project")
            .wait()
            .unwrap();

        assert!(exit_code.success(), "Couldn't build the project");
    }

    eprintln!("Loading original game...");

    let buf = iso::reader::load_iso_buf(&config.src.iso)
        .unwrap_or_else(|_| panic!("Couldn't find \"{}\".", config.src.iso.display()));

    let mut iso = iso::reader::load_iso(&buf);

    eprintln!("Replacing files...");

    for (iso_path, actual_path) in &config.files {
        iso.resolve_and_create_path(iso_path).data = fs::read(actual_path)
            .unwrap_or_else(|_| {
                panic!(
                    "Couldn't read the file \"{}\" to store it in the ISO.",
                    actual_path.display()
                )
            })
            .into();
    }

    let mut original_symbols = HashMap::new();
    if let Some(framework_map) = config.src.map.as_ref().and_then(|m| iso.resolve_path(m)) {
        eprintln!("Parsing game's map...");
        original_symbols = framework_map::parse(&framework_map.data);
    } else {
        eprintln!("No symbol map specified or it wasn't found.");
    }

    eprintln!("Linking...");

    let mut libs_to_link = Vec::with_capacity(config.src.link.len() + 1);
    for lib_path in &config.src.link {
        let mut file_buf = fs::read(&config.src.link[0]).unwrap_or_else(|_| {
            panic!(
                "Couldn't load \"{}\". Did you build the project correctly?",
                lib_path.display()
            )
        });
        libs_to_link.push(file_buf);
    }
    libs_to_link.push(linker::BASIC_LIB.to_owned());

    let linked = linker::link(
        &libs_to_link,
        base_address.value() as u32,
        config.link.entries.clone(),
        &original_symbols,
    );

    eprintln!("Creating map...");

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
    );

    let mut instructions = Vec::new();
    if let Some(patch) = config.src.patch.take() {
        eprintln!("Parsing patch...");

        let mut asm = String::new();
        File::open(&patch)
            .unwrap_or_else(|_| panic!("Couldn't find \"{}\".", patch.display()))
            .read_to_string(&mut asm)
            .expect("Couldn't read the patch file");

        let lines = &asm.lines().collect::<Vec<_>>();

        let mut assembler = Assembler::new(linked.symbol_table);
        instructions = assembler.assemble_all_lines(lines);
    }

    {
        eprintln!("Patching game...");

        let main_dol = iso.main_dol_mut().expect("Dol file not found");

        let original = DolFile::parse(&main_dol.data);
        main_dol.data = patch_game(original, linked.dol, &instructions).into();
    }
    {
        eprintln!("Patching banner...");

        if let Some(banner_file) = iso.banner_mut() {
            // TODO Not always true
            let is_japanese = true;
            let mut banner = Banner::parse(is_japanese, &banner_file.data);
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
                    .expect("Couldn't open banner image")
                    .to_rgba();
                banner.image.copy_from_slice(&image);
            }
            banner_file.data = banner.to_bytes(is_japanese).to_vec().into();
        } else {
            eprintln!("No banner to patch.");
        }
    }

    eprintln!("Building ISO...");
    let iso_path = &config.build.iso;
    iso::writer::write_iso(
        BufWriter::with_capacity(4 << 20, File::create(iso_path).unwrap()),
        &iso,
    ).unwrap();
}

fn new(name: String) {
    let exit_code = Command::new("cargo")
        .args(&["new", "--lib", &name])
        .spawn()
        .expect("Couldn't create the cargo project")
        .wait()
        .unwrap();

    assert!(exit_code.success(), "Couldn't create the cargo project");

    let mut file = File::create(format!("{}/RomHack.toml", name)).unwrap();
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
    ).unwrap();

    let mut file = File::create(format!("{}/src/lib.rs", name)).unwrap();
    write!(
        file,
        "{}",
        r#"#![no_std]
#![feature(lang_items)]
pub mod lang_items;

#[no_mangle]
pub extern "C" fn init() {}
"#
    ).unwrap();

    let mut file = File::create(format!("{}/src/lang_items.rs", name)).unwrap();
    write!(
        file,
        "{}",
        r#"#[cfg_attr(any(target_arch = "powerpc", target_arch = "wasm32"), lang = "panic_fmt")]
#[no_mangle]
pub extern "C" fn panic_fmt() -> ! {
    loop {}
}
"#
    ).unwrap();

    let mut file = File::create(format!("{}/src/patch.asm", name)).unwrap();
    write!(
        file,
        r#"; You can use this to patch the game's code to call into the Rom Hack's code
"#
    ).unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(format!("{}/Cargo.toml", name))
        .unwrap();
    write!(
        file,
        r#"
[lib]
crate-type = ["staticlib"]

[profile.release]
panic = "abort"
lto = true
"#
    ).unwrap();
}

fn main() {
    let opt = Opt::from_args();

    match opt {
        Opt::Build {} => build(),
        Opt::New { name } => new(name),
    }
}

fn patch_game(
    mut original: DolFile,
    intermediate: DolFile,
    instructions: &[Instruction],
) -> Vec<u8> {
    original.append(intermediate);
    original.patch(instructions);

    original.to_bytes()
}
