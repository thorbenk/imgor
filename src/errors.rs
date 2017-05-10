// Copyright 2017 Thorben Kroeger.
// Dual-licensed MIT and Apache 2.0 (see LICENSE files for details).

error_chain!{
    foreign_links {
        Io(::std::io::Error);
        Rexiv2(::rexiv2::Rexiv2Error);
    }

    errors {
        PathNotUtf8(path: ::std::path::PathBuf) {
            description("path is not valid utf-8")
            display("path '{}' is not valid utf-8", path.display())
        }
    }
}