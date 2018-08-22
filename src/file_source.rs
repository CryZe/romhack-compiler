use failure::{err_msg, Error};
use image::{self, DynamicImage};
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use zip::ZipArchive;

pub trait FileSource {
    fn read_to_vec<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>, Error>;
    fn read_to_string<P: AsRef<Path>>(&mut self, path: P) -> Result<String, Error>;
    fn open_image<P: AsRef<Path>>(&mut self, path: P) -> Result<DynamicImage, Error>;
}

pub struct FileSystem;

impl FileSource for FileSystem {
    fn read_to_vec<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>, Error> {
        Ok(fs::read(path)?)
    }
    fn read_to_string<P: AsRef<Path>>(&mut self, path: P) -> Result<String, Error> {
        Ok(fs::read_to_string(path)?)
    }
    fn open_image<P: AsRef<Path>>(&mut self, path: P) -> Result<DynamicImage, Error> {
        Ok(image::open(path)?)
    }
}

impl<R: Read + Seek> FileSource for ZipArchive<R> {
    fn read_to_vec<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>, Error> {
        let mut file = self.by_name(
            path.as_ref()
                .to_str()
                .ok_or_else(|| err_msg("Invalid path"))?,
        )?;
        let mut buf = Vec::with_capacity(file.size() as usize + 1);
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }
    fn read_to_string<P: AsRef<Path>>(&mut self, path: P) -> Result<String, Error> {
        let mut file = self.by_name(
            path.as_ref()
                .to_str()
                .ok_or_else(|| err_msg("Invalid path"))?,
        )?;
        let mut buf = String::with_capacity(file.size() as usize + 1);
        file.read_to_string(&mut buf)?;
        Ok(buf)
    }
    fn open_image<P: AsRef<Path>>(&mut self, path: P) -> Result<DynamicImage, Error> {
        let buf = self.read_to_vec(path)?;
        Ok(image::load_from_memory(&buf)?)
    }
}
