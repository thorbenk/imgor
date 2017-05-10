// Copyright 2017 Thorben Kroeger.
// Dual-licensed MIT and Apache 2.0 (see LICENSE files for details).

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use metadata::Metadata;
use errors::*;

static MEDIA_EXTENSIONS: &[&str] = &["cr2", "jpg", "jpeg", "mov", "xmp"];

#[derive(Debug, PartialEq, Eq)]
struct File {
    path: PathBuf,
    derived_from: Option<PathBuf>
}

impl File {
    fn is_source(&self) -> bool {
        self.derived_from.is_none()
    }
}

#[cfg(test)]
macro_rules! media_file_vec {
    ( $( $x:expr => $y:expr), * ) => {
        {
            let mut v = Vec::new();
            $(
                v.push(File {  
                    path: PathBuf::from($x),
                    derived_from: $y.map(|e: &str| PathBuf::from(e))
                });
            )*
            v
        }
    }
}

fn classify_files_impl<F>(paths: &Vec<PathBuf>, derived_from: F) -> Result<Vec<File>>
where
    F: Fn(&Path) -> Option<PathBuf>,
{
    Ok(paths.iter()
        .filter(|path| {
            match path.extension() {
                Some(e) => {
                    let lowercase_ext = e.to_str()
                        //.ok_or(ErrorKind::PathNotUtf8(path.to_path_buf()))? // FIXME
                        .unwrap()
                        .to_lowercase();
                    MEDIA_EXTENSIONS.iter().any(|&e| e == lowercase_ext)
                }
                None => {
                    // skip file without extension
                    false
                }
            }
        })
        .map(|path| {
            File { path: path.clone(), derived_from: derived_from(&path) }
        }).collect())
}

#[test]
fn test_classify_files() {
    let paths = vec!["/a/1.jpg", "/a/1.cr2", "/a/x.mov", "/a/1.xmp", "/a/1.txt", "/a/2.JPG"]
        .iter()
        .map(|&e| PathBuf::from(e))
        .collect::<Vec<_>>();
    let a = classify_files_impl(&paths, |path| {
        if path == PathBuf::from("/a/1.jpg") || path == PathBuf::from("/a/1.xmp") {
            Some(PathBuf::from("/a/1.cr2"))
        } else {
            None
        }
    });
    let e = media_file_vec![
        "/a/1.jpg" => Some("/a/1.cr2"),
        "/a/1.cr2" => None,
        "/a/x.mov" => None,
        "/a/1.xmp" => Some("/a/1.cr2"),
        "/a/2.JPG" => None
    ];
    assert_eq!(a.unwrap(), e);
}

fn classify_files(paths: &Vec<PathBuf>) -> Result<Vec<File>> {
    classify_files_impl(&paths, |path| {
        let meta = Metadata::new(&path);
        match meta {
            Ok(m) => {
                m.derived_from()
            }    
            Err(_) => {
                // cannot obtain `DerivedFrom` from .MOV file for example
                // (or processed images without metadata)
                None
            }
        }
    })
}

// Represents a single photo file (e.g. a RAW file) together with
// - any XMP sidecar files that may reference it (via XMP's DerivedFrom)
// - any JPG files that may reference it (via XMP's DerivedFrom)
#[derive(Debug, Clone, PartialEq)]
pub struct Photo {
    pub source: PathBuf,
    pub derived: Vec<PathBuf>,
}

impl Photo {
    pub fn new(file: PathBuf) -> Photo {
        Photo {
            source: file,
            derived: Vec::<PathBuf>::new(),
        }
    }

    pub fn add_derived(&mut self, file: PathBuf) {
        self.derived.push(file);
    }
}


fn group_photo_files_impl(files: &Vec<File>) -> Vec<Photo> {
    // 1.) first, create `Photo` instances for each RAW file found
    // 2.) associate all XMP and JPG files with the `Photo` instance
    //     which has the corresponding RAW file as `Photo::source`
    // 3.) remaining files become `Photo` instances of their own

    let mut h = HashMap::<&Path, Photo>::new();

    let mut files_used = vec![false; files.len()];

    // add all raw files
    for (ref file, ref mut used) in izip!(files, &mut files_used) {
        if file.is_source() {
            h.insert(&file.path, Photo::new(file.path.clone()));
            **used = true;
        }
    }

    // associate derived files with their corresponding source files
    for (ref file, ref mut used) in izip!(files, &mut files_used) {
        if !file.is_source() {
            let derived_from = file.derived_from.as_ref().unwrap();
            h.get_mut(derived_from.as_path())
                .expect(&format!("referenced image {:?} does not exist", file.derived_from))
                .add_derived(file.path.clone());
            **used = true;
        }
    }

    // flatten and sort by source path
    let mut result = h.values().cloned().collect::<Vec<Photo>>();
    result.sort_by(|ref a, ref b| a.source.cmp(&b.source));
    result
}

#[cfg(test)]
macro_rules! photo {
    (
        $x:expr; [ $( $y:expr ),* ]
    ) => {
        {
            let source = PathBuf::from($x);
            let derived : Vec<&str> = vec![$($y),*];
            Photo {
                source: source,
                derived: derived.iter().map(|e: &&str| PathBuf::from(e)).collect()
            }
        }
    }
}

#[test]
fn test_group_photo_files_impl_1() {
    // associate JPG and RAW file by common basename
    let f = media_file_vec![
        "/a/3.JPG"   => Some("/a/3.cr2"),
        "/a/3.cr2"   => None,
        "/a/1.jpg"   => Some("/a/1.CR2"),
        "/a/1.CR2"   => None,
        "/a/2.JPG"   => Some("/a/2.CR2"),
        "/a/2.CR2"   => None,
        "/a/4.CR2"   => None,
        "/a/b/4.JPG" => None
    ];

    let a = group_photo_files_impl(&f);

    // 1.) output is sorted
    // 2.) associations RAW <-> JPG are correct
    let e = vec![
        photo!["/a/1.CR2"; ["/a/1.jpg"]],
        photo!["/a/2.CR2"; ["/a/2.JPG"]],
        photo!["/a/3.cr2"; ["/a/3.JPG"]],
        photo!["/a/4.CR2"; []],
        photo!["/a/b/4.JPG"; []],
    ];
    assert_eq!(a, e);
}

#[test]
fn test_group_photo_files_impl_2() {
    let f = media_file_vec![
        "/a/1.jpg"    => Some("/a/1.cr2"),
        "/a/1.cr2"    => None,
        "/a/2.mov"    => None,
        "/a/1.xmp"    => Some("/a/1.cr2"),
        "/a/1_v2.xmp" => Some("/a/1.cr2"),
        "/a/3.jpg"    => None
    ];

    let a = group_photo_files_impl(&f);
    let e = vec![
        photo!("/a/1.cr2"; ["/a/1.jpg", "/a/1.xmp", "/a/1_v2.xmp"]),
        photo!("/a/2.mov"; []),
        photo!("/a/3.jpg"; [])
    ];

    assert_eq!(a, e);
}

pub fn group_photo_files(files: &Vec<PathBuf>) -> Result<Vec<Photo>> {
    let classified = classify_files(&files)?;
    Ok(group_photo_files_impl(&classified))
}