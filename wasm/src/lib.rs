extern crate failure;
extern crate romhack_backend;

use failure::Error;
use romhack_backend::{
    build_iso, iso::writer::write_iso, open_config_from_patch, KeyValPrint, MessageKind,
};
use std::alloc::{alloc as allocate, dealloc as deallocate, Layout};
use std::io::{self, BufWriter, Cursor, SeekFrom, Write};
use std::slice::from_raw_parts;

extern "C" {
    fn count_write(buf_len: usize);
    fn count_seek(kind: u8, offset: isize) -> usize;
    fn restart();
    fn write(buf_ptr: *const u8, buf_len: usize);
    fn seek(kind: u8, offset: isize) -> usize;
    fn key_val_print(kind: u8, key: *const u8, key_len: usize, val: *const u8, val_len: usize);
    fn set_name(ptr: *const u8, len: usize);
    fn error(ptr: *const u8, len: usize);
}

struct JSPrinter;

impl KeyValPrint for JSPrinter {
    fn print(&self, kind: Option<MessageKind>, key: &str, val: &str) {
        unsafe {
            let kind = match kind {
                Some(MessageKind::Error) => 2,
                Some(MessageKind::Warning) => 1,
                None => 0,
            };
            key_val_print(kind, key.as_ptr(), key.len(), val.as_ptr(), val.len());
        }
    }
}

struct RomHackWriter;

impl io::Write for RomHackWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            write(buf.as_ptr(), buf.len());
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Seek for RomHackWriter {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = unsafe {
            match pos {
                SeekFrom::Start(offset) => seek(0, offset as isize),
                SeekFrom::End(offset) => seek(1, offset as isize),
                SeekFrom::Current(offset) => seek(2, offset as isize),
            }
        };
        Ok(new_pos as u64)
    }
}

struct RomHackCounter;

impl io::Write for RomHackCounter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            count_write(buf.len());
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Seek for RomHackCounter {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = unsafe {
            match pos {
                SeekFrom::Start(offset) => count_seek(0, offset as isize),
                SeekFrom::End(offset) => count_seek(1, offset as isize),
                SeekFrom::Current(offset) => count_seek(2, offset as isize),
            }
        };
        Ok(new_pos as u64)
    }
}

#[no_mangle]
pub unsafe extern "C" fn alloc(size: usize) -> *mut u8 {
    allocate(Layout::from_size_align_unchecked(size, 1))
}

#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    deallocate(ptr, Layout::from_size_align_unchecked(size, 1))
}

#[no_mangle]
pub unsafe extern "C" fn create_romhack(
    patch_ptr: *const u8,
    patch_len: usize,
    iso_ptr: *const u8,
    iso_len: usize,
) -> bool {
    let patch = from_raw_parts(patch_ptr, patch_len);
    let iso = from_raw_parts(iso_ptr, iso_len);
    if let Err(e) = try_create_romhack(patch, iso) {
        let mut buf = Vec::new();
        for cause in e.iter_chain() {
            buf.clear();
            write!(buf, "{}", cause).unwrap();
            error(buf.as_ptr(), buf.len());
        }
        false
    } else {
        true
    }
}

fn try_create_romhack(patch: &[u8], iso: &[u8]) -> Result<(), Error> {
    let (zip, compiled_library, mut config) = open_config_from_patch(Cursor::new(patch))?;
    if let Some(name) = &config.info.game_name {
        unsafe {
            set_name(name.as_ptr(), name.len());
        }
    }
    let romhack = build_iso(&JSPrinter, zip, iso, compiled_library, &mut config)?;
    JSPrinter.print(None, "Measuring", "Rom Hack File Size");
    write_iso(RomHackCounter, &romhack)?;
    unsafe {
        restart();
    }
    JSPrinter.print(None, "Writing", "Rom Hack");
    let writer = BufWriter::new(RomHackWriter);
    write_iso(writer, &romhack)
}
