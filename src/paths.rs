// Copyright 2017 Thorben Kroeger.
// Dual-licensed MIT and Apache 2.0 (see LICENSE files for details).

use std::path::{Path, PathBuf};

pub struct CommonPrefix {
    pub prefix: PathBuf,
    pub suffix1: PathBuf,
    pub suffix2: PathBuf
}

pub fn common_prefix(path1: &Path, path2: &Path) -> CommonPrefix {
    let mut a = path1.components().peekable();
    let mut b = path2.components().peekable();
    
    let mut common_prefix = vec![];
    loop {
        match (a.peek(), b.peek()) {
            (Some(&e1), Some(&e2)) => {
                if e1 != e2 {
                    break;
                }
                common_prefix.push(e1);
                a.next();
                b.next();
               continue;
            }
            _ => { break; }
        }
    }
    let common_prefix = common_prefix.iter().map(|s| s.as_os_str()).collect::<PathBuf>();
    let r1 = a.map(|s| s.as_os_str()).collect::<PathBuf>();
    let r2 = b.map(|s| s.as_os_str()).collect::<PathBuf>();

    CommonPrefix {
        prefix: common_prefix,
        suffix1: r1,
        suffix2: r2
    }
}