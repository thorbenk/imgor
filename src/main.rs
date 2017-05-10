// Copyright 2017 Thorben Kroeger.
// Dual-licensed MIT and Apache 2.0 (see LICENSE files for details).

extern crate chrono;
extern crate rexiv2;
extern crate clap;
extern crate imgor;
extern crate error_chain;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::cmp::Ordering;
use clap::{Arg, App, SubCommand};
use chrono::{UTC, DateTime};

use imgor::*;
use metadata::{extract_datetime};

#[derive(Debug, PartialEq)]
enum Cmd {
    CreateDirectory(PathBuf),
    Rename(PathBuf, PathBuf),
    AdjustRef(PathBuf, PathBuf),
}

fn collect_files(dirname: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(dirname)?;

    let mut paths = Vec::<PathBuf>::new();
    for dir_entry in entries {
        let path = dir_entry?.path();
        if path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

struct RawMeta {
    datetime_original: Option<DateTime<UTC>>,
}

struct AnnotatedPhoto {
    photo: Photo,
    meta: RawMeta,
}

fn extract_raw_meta(photo: &Photo) -> RawMeta {
    RawMeta { datetime_original: extract_datetime(&photo.source) }
}

fn date_photo_files(files: &Vec<Photo>) -> Vec<AnnotatedPhoto> {
    files
        .iter()
        .map(
            |f| {
                let meta = extract_raw_meta(&f);
                AnnotatedPhoto {
                    photo: f.clone(),
                    meta: meta,
                }
            }
        )
        .collect()
}

/// replaces `old` with `new` in `file_name`s stem, and returns
/// the new filename with lowercased extensions
fn make_new_filename(file_name: &str, old: &str, new: &str) -> String {
    // note: because a file (like 1.cr2.xmp) may have multiple extensions
    //       we search for "." ourselves
    let first_dot = file_name.find(".");
    match first_dot {
        Some(index) => {
            let (ref stem, ref ext) = file_name.split_at(index);
            let new_stem = stem.replace(&old, &new);
            let new_ext = ext.to_lowercase();

            format!("{}{}", new_stem, new_ext)
        },
        None => {
            // there is no extension
            file_name.replace(&old, &new)
        }
    }
}

#[test]
fn test_make_new_filename() {

    let inputs = vec!["my_file.JPG", "my_file.CR2.JPG", "my_file.cr2.JPG", "my_file"];
    let old = "my_file";
    let new = "0000";

    let a : Vec<_> = inputs.iter()
        .map(|e| make_new_filename(e, &old, &new)).collect();
    let e = vec!["0000.jpg", "0000.cr2.jpg", "0000.cr2.jpg", "0000"];

    assert_eq!(a, e);
}

fn create_move_commands(photo: &Photo, new_stem: &str, out_dir: &Path) -> imgor::Result<Vec<Cmd>> {
    let mut cmds : Vec<Cmd> = Vec::new();

    let source_stem = &photo.source.file_stem()
        .ok_or(format!("file `{}` has no basename", photo.source.display()))?
        .to_str()
        .ok_or(ErrorKind::PathNotUtf8(photo.source.clone()))?;
    
    let source_file_name = &photo.source.file_name()
        .expect("need filename")
        .to_str()
        .ok_or(ErrorKind::PathNotUtf8(photo.source.clone()))?;

    let new_source = make_new_filename(&source_file_name, &source_stem, &new_stem);
    let new_source_file = &out_dir.join(&new_source);

    cmds.push(Cmd::Rename(photo.source.clone(), new_source_file.clone()));

    for derived in &photo.derived {
        let derived_file_name = &derived.file_name()
            .expect("need filename")
            .to_str()
            .ok_or(ErrorKind::PathNotUtf8(derived.clone()))?;

        let new_derived_file = &out_dir.join(
            make_new_filename(&derived_file_name, source_stem, new_stem));

        cmds.push(Cmd::Rename(derived.clone(), new_derived_file.clone()));
        cmds.push(
            Cmd::AdjustRef(new_derived_file.clone(), new_source_file.clone())
        );
    }

    Ok(cmds)
}

#[test]
fn test_create_move_commands() {
    let p = Photo {
        source: PathBuf::from("/a/1.CR2"),
        derived: vec![PathBuf::from("/a/1.cr2.xmp"), PathBuf::from("/a/1_v2.CR2.xmp"), PathBuf::from("/a/1.jpg")]
    };
    let out_dir = PathBuf::from("/tmp");
    let a = create_move_commands(&p, &"x", &out_dir);
    let e = vec![
        Cmd::Rename(p.source.clone(),           out_dir.join("x.cr2")),
        Cmd::Rename(p.derived[0].clone(),      out_dir.join("x.cr2.xmp")),
        Cmd::AdjustRef(out_dir.join("x.cr2.xmp"), out_dir.join("x.cr2")),
        Cmd::Rename(p.derived[1].clone(),      out_dir.join("x_v2.cr2.xmp")),
        Cmd::AdjustRef(out_dir.join("x_v2.cr2.xmp"), out_dir.join("x.cr2")),
        Cmd::Rename(p.derived[2].clone(), out_dir.join("x.jpg")),
        Cmd::AdjustRef(out_dir.join("x.jpg"), out_dir.join("x.cr2")),
    ];
    assert_eq!(a.unwrap(), e);
}

fn group_files_by_date(in_dir: &Path, out_dir: &Path) -> imgor::Result<Vec<Cmd>> {
    let files = collect_files(&in_dir)?;
    let grouped = group_photo_files(&files)?;
    let mut dated = date_photo_files(&grouped);

    dated.sort_by(
        |ref a, ref b| match (a.meta.datetime_original, b.meta.datetime_original) {
            (Some(d1), Some(d2)) => d1.cmp(&d2),
            (Some(_d), None) => Ordering::Greater,
            (None, Some(_d)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        }
    );

    // group by date
    let groups = group_by_fn(
        &dated, |a, b| {
            match (a.meta.datetime_original, b.meta.datetime_original) {
                (Some(d1), Some(d2)) => d1.date() == d2.date(),
                (Some(_), None) => false,
                (None, Some(_)) => false,
                (None, None) => true
            }
        }
    );

    let mut cmds = vec![];

    for group in groups {
        let group_name = match group[0].meta.datetime_original {
            Some(d) => format!("{}", d.date().format("%Y-%m-%d")),
            None => "no-date".into()
        };
        let group_dir = out_dir.join(&group_name);

        cmds.push(Cmd::CreateDirectory(out_dir.join(&group_name)));

        for (i, f) in group.iter().enumerate() {
            let new_stem = format!("{:04}_{}", i, group_name);
            let mut c = create_move_commands(&f.photo, &new_stem, &group_dir)?;
            cmds.append(&mut c);
        }
    }

    Ok(cmds)
}

fn print_rename(src: &Path, dest: &Path) -> String {
    let c = common_prefix(&src, &dest);
    format!("{}/{{{} => {}}}", c.prefix.display(), c.suffix1.display(), c.suffix2.display())
}

fn run() -> imgor::Result<()> {
    let matches = App::new("imgor")
        .version("0.01")
        .author("Thorben Kroeger <thorbenkroeger@gmail.com>")
        .about("command line file management for (raw) photos and associated sidecar files")
        .arg(Arg::with_name("dry run")
            .short("n")
            .long("dry-run")
            .help("only print which commands would be executed"))
        .subcommand(SubCommand::with_name("group")
            .about("sort photos into groups")
            .arg(Arg::with_name("DIRECTORY")
                .help("directory containing the photos to be grouped")
                .required(true)
                .index(1)))
        .get_matches();

    let dry_run = matches.is_present("dry run");

    if let Some(matches) = matches.subcommand_matches("group") {
        let from_dir = PathBuf::from(matches.value_of("DIRECTORY").unwrap());
        let to_dir = from_dir.join("grouped");

        let cmds = group_files_by_date(&from_dir, &to_dir)?;
        if dry_run {
            for cmd in cmds {
                match cmd {
                    Cmd::Rename(ref src, ref dest) => {
                        println!("rename     {}", print_rename(&src, &dest));
                    },
                    Cmd::CreateDirectory(dir) => {
                        println!("create dir {}", dir.display());
                    },
                    Cmd::AdjustRef(ref file, ref referenced_image) => {
                        let c = common_prefix(&file, &referenced_image);
                        assert!(c.suffix1.components().count() == 1);
                        assert!(c.suffix2.components().count() == 1);
                        println!("adjust ref {} --> {}", file.display(), c.suffix2.display());
                    }
                }
            }
        } else {
            for cmd in cmds {
                match cmd {
                    Cmd::Rename(ref src, ref dest) => {
                        std::fs::copy(&src, &dest)?;
                    },
                    Cmd::CreateDirectory(dir) => {
                        if dir.exists() {
                            panic!();
                        }
                        std::fs::create_dir_all(&dir)?;
                    },
                    Cmd::AdjustRef(ref file, ref referenced_image) => {
                        let c = common_prefix(&file, &referenced_image);
                        assert!(c.suffix1.components().count() == 1);
                        assert!(c.suffix2.components().count() == 1);
                        let derived_from = c.suffix2.to_str()
                            .ok_or(ErrorKind::PathNotUtf8(c.suffix2.clone()))?;
                        write_derivedfrom(&file, &derived_from);
                    }
                }
            }
        }
    }

    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        use error_chain::ChainedError;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";
        writeln!(stderr, "{}", e.display()).expect(errmsg);
        ::std::process::exit(1);
    }
}
