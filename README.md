## imgor - command line image organizer 

This is a simple command line utility for managing photo files.
It assumes a RAW workflow with [Darktable](http://www.darktable.org/).

**Note:** This is work in progress. I'm writing this in `rust` as a learning
exercise.

Consider the following group of files, which consist of the original
RAW file and various derivatives:
```bash
img.cr2        # the raw file
img.cr2.xmp    # a sidecar file, describing a processing of `img.cr2`
img_v2.cr2.xmp # a 2nd sidecar file, describing a different processing
img.jpg        # "developed" image (via the instructions in `img.cr2.xmp`)
```
The derivatives point back to the raw file via the `DerivedFrom` attribute
in their XMP metadata.

When doing file management operations (copy, move),
`imgor` helps you to rename these files consistently. It
- renames the files
- adjusts the `DerivedFrom` XMP metadata

## Examples

```bash
# copy files, organizing them into subfolders corresponding to the
# day the photos were shot:
imgor --dry-run group /photos/unsorted_photos
```

## Compilation

Developed with rust nightly.

On Ubuntu, you proabbly need the following additional packages:
```
sudo apt install pkg-config libgexiv2-dev
```

## License

This project is distributed under the terms of both the MIT license
and the Apache License (Version 2.0).
See `LICENSE-MIT` and `LICENSE-APACHE` for details.
