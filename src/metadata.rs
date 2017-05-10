// Copyright 2017 Thorben Kroeger.
// Dual-licensed MIT and Apache 2.0 (see LICENSE files for details).

extern crate chrono;
extern crate rexiv2;

use std::process::Command;
use std::path::{Path, PathBuf};
use std::ascii::AsciiExt;

#[cfg(test)]
use std::env;
#[cfg(test)]
use std::ffi::OsStr;

use chrono::offset::TimeZone;
use chrono::{UTC, DateTime};

use errors::Result;

static REXIV2_EXTENSIONS: &[&str] = &["jpg", "cr2"];
static EXIFTOOL_EXTENSIONS: &[&str] = &["mov"];

static XMP_XMPMM_DERIVEDFROM: &str = &"Xmp.xmpMM.DerivedFrom";
static XMP_XMP_RATING: &str = &"Xmp.xmp.Rating";
static XMP_DARKTABLE_COLORLABELS: &str = &"Xmp.darktable.colorlabels";
static EXIF_PHOTO_DATETIMEORIGINAL: &str = &"Exif.Photo.DateTimeOriginal";

#[derive(Debug)]
pub enum DarktableColor {
    Red,
    Yellow,
    Green,
    Blue,
    Magenta,
}

pub struct Metadata {
    path: PathBuf,
    meta: rexiv2::Metadata
}

fn parse_exif_datetime(datetime: &str) -> DateTime<UTC> {
    // http://www.awaresystems.be/imaging/tiff/tifftags/privateifd/exif/datetimeoriginal.html
    // YYYY:MM:DD HH:MM:SS
    chrono::UTC
        .datetime_from_str(datetime, "%Y:%m:%d %H:%M:%S")
        .unwrap()
}

impl Metadata {
    pub fn new(file: &Path) -> Result<Metadata> {
        let meta = rexiv2::Metadata::new_from_path(&file)?;
        Ok(Metadata {
            path: file.to_path_buf(),
            meta: meta
        })
    }

    // -1 means rejected
    pub fn rating(&self) -> Option<i32> {
        self.meta.get_tag_string(&XMP_XMP_RATING)
            .unwrap()
            .parse::<i32>().ok()
    }

    pub fn darktable_colorlabels(&self) -> Option<Vec<DarktableColor>> {
        let colors = self.meta.get_tag_string(&XMP_DARKTABLE_COLORLABELS).unwrap();
        let colors: Vec<i32> = colors
            .split(',')
            .map(str::trim)
            .map(|s| s.parse::<i32>().unwrap())
            .collect();

        Some(colors.iter().map(
            |number| match *number {
                0 => DarktableColor::Red,
                1 => DarktableColor::Yellow,
                2 => DarktableColor::Green,
                3 => DarktableColor::Blue,
                4 => DarktableColor::Magenta,
                _ => panic!(),
            })
            .collect())
    }

    pub fn datetime_original(&self) -> Option<DateTime<UTC>> {
        self.meta.get_tag_string(&EXIF_PHOTO_DATETIMEORIGINAL)
            .ok().map(|d| parse_exif_datetime(&d))
    }
    
    pub fn derived_from(&self) -> Option<PathBuf> {
        let file_dir = self.path.parent().unwrap();
        let file_str = self.path.to_str().unwrap();

        let meta = try_opt!(rexiv2::Metadata::new_from_path(&file_str).ok());
        
        meta.get_tag_string(&XMP_XMPMM_DERIVEDFROM).ok().map(
            |derived_from| file_dir.join(&derived_from)
        )
    }
}

fn run_exiftool_and_get_create_date(file: &str) -> Option<DateTime<UTC>> {
    // rexiv2 apparently does not deal with .MOV files
    // We use the commandline `exiftool` to get at the information
    let output = Command::new("exiftool")
        .arg("-DateTimeOriginal")
        .arg("-S")
        .arg(&file)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let sep = stdout.find(":").unwrap();
    let newline = stdout.rfind("\n").unwrap();
    let datetime = &stdout[sep + 2..newline];
    Some(parse_exif_datetime(&datetime))
}

pub fn write_derivedfrom(file: &Path, derived_from: &str) {
    let file_str = file.to_str().unwrap();
    let meta = rexiv2::Metadata::new_from_path(&file_str).unwrap();
    meta.set_tag_string(&XMP_XMPMM_DERIVEDFROM, derived_from).unwrap();
    meta.save_to_file(&file).unwrap();
}

#[cfg(test)]
fn get_target_dir() -> PathBuf {
    // see: https://github.com/rust-lang/cargo/issues/2841
    let bin = env::current_exe().expect("exe path");
    let mut target_dir = PathBuf::from(bin.parent().expect("bin parent"));
    while target_dir.file_name() != Some(OsStr::new("target")) {
        target_dir.pop();
    }
    target_dir
}

#[test]
fn test_extract_derivedfrom() {
    let base = get_target_dir().parent().unwrap().join("test_data");

    let xmp_file = base.join("IMG_7506.CR2.xmp");
    let derivedfrom_file = base.join("IMG_7506.CR2");
    let meta = Metadata::new(&xmp_file).unwrap();
    let d = meta.derived_from();

    assert_eq!(d.unwrap(), derivedfrom_file);
}

pub fn extract_datetime(path: &Path) -> Option<DateTime<UTC>> {
    let ext = path.extension()
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    let path_str = path.to_str().unwrap();

    if REXIV2_EXTENSIONS.iter().any(|&e| e == ext) {
        let meta = Metadata::new(&path).unwrap();
        return meta.datetime_original()
    } else if EXIFTOOL_EXTENSIONS.iter().any(|&e| e == ext) {
        return run_exiftool_and_get_create_date(path_str);
    }
    panic!();
}
