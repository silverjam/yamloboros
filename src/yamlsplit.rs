/*
* Copyright (C) 2024 Jason Mobarak
*
* Contact: Jason Mobarak <git@jason.mobarak.name>
*
* This source is subject to the license found in the file 'LICENSE' which must
* be be distributed together with this source. All other rights reserved.
*
* THIS CODE AND INFORMATION IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND,
* EITHER EXPRESSED OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE IMPLIED
* WARRANTIES OF MERCHANTABILITY AND/OR FITNESS FOR A PARTICULAR PURPOSE.
*/

use std::boxed::Box;
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::OnceLock;

use regex::Regex;

type Result<T> = anyhow::Result<T, Box<dyn Error>>;

fn regex_doc_start() -> &'static Regex {
    const REGEX_PAT: &str = "^--- *$";
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(REGEX_PAT).unwrap())
}

fn regex_doc_end() -> &'static Regex {
    const REGEX_PAT: &str = "^\\.\\.\\. *$";
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(REGEX_PAT).unwrap())
}

/// Returns a boxed reader for either stdin or the input file, also returns the
/// basename and extension of the input file.
fn stdin_or_input_file() -> Result<(Box<dyn io::Read>, String, String)> {
    let mut args = std::env::args();
    let input_filename = args.nth(1).unwrap_or_else(|| "-".to_string());
    let input_file = match input_filename.as_str() {
        "-" => Box::new(io::stdin()) as Box<dyn io::Read>,
        _ => Box::new(std::fs::File::open(&input_filename)?) as Box<dyn io::Read>,
    };
    let input_filename = if input_filename == "-" {
        "stdin.yaml".to_string()
    } else {
        input_filename
    };
    let (basename, extension) = basename(PathBuf::from(input_filename));
    Ok((input_file, basename, extension))
}

/// Get the basename of a file without the extension.  Includes
/// any leading path components.  The extension is defined as anything
/// after the last "." in the filename.  Also returns the extension;
fn basename(filename: PathBuf) -> (String, String) {
    let dirname = filename.parent().unwrap().to_str().unwrap();
    let extension = filename.extension();
    let extension = extension.unwrap_or_default().to_str().unwrap().to_string();
    let dirname = if dirname.is_empty() {
        "".to_string()
    } else {
        dirname.to_string() + "/"
    };
    let filename = filename.file_name().unwrap().to_str().unwrap();
    // Get everything before the last ".".
    let split = filename.rsplit_once('.');
    if let Some((basename, _)) = split {
        (dirname + basename, extension)
    } else {
        (dirname + filename, extension)
    }
}

#[test]
fn test_basename() {
    assert_eq!(
        basename(PathBuf::from("foo")),
        ("foo".to_string(), "".to_string())
    );
    assert_eq!(
        basename(PathBuf::from("foo.yaml")),
        ("foo".to_string(), "yaml".to_string())
    );
    assert_eq!(
        basename(PathBuf::from("foo.bar.yaml")),
        ("foo.bar".to_string(), "yaml".to_string())
    );
    assert_eq!(
        basename(PathBuf::from("foo.bar.baz.yaml")),
        ("foo.bar.baz".to_string(), "yaml".to_string())
    );
    assert_eq!(
        basename(PathBuf::from("dir/foo.bar.baz.yaml")),
        ("dir/foo.bar.baz".to_string(), "yaml".to_string())
    );
}

fn output_line_to_file(
    line: &str,
    output_file: &mut Option<io::BufWriter<std::fs::File>>,
) -> Result<()> {
    if let Some(output_file) = output_file {
        output_file.write_all(line.as_bytes())?;
        output_file.write_all(b"\n")?;
    }
    Ok(())
}

fn open_new_file_for_output(
    basename: &str,
    extension: &str,
    output_file_count: &mut u32,
) -> Result<io::BufWriter<std::fs::File>> {
    let output_filename = format!("{}-{}.{}", basename, output_file_count, extension);
    let output_file = std::fs::File::create(output_filename)?;
    *output_file_count += 1;
    Ok(io::BufWriter::new(output_file))
}

fn main() -> Result<()> {
    /*
       Open a file from the command line and splits the YAML documents into
       separate files based on the appearance of the YAML document separator
       "---".

       Each file will be named after the basename of the input file with a
       numeric suffix.  For example, if the input file is named "foo.yaml", the
       output files will be named "foo-1.yaml".

       For the first document, the "---" separator is optional.  If it is
       omitted, the first document number will default to "1". For example, if
       the input file is named "foo.yaml", the output file will be named
       "foo-1.yaml".
    */
    // println!("1");
    let (input_file, basename, extension) = stdin_or_input_file()?;
    // println!("2");
    // let mut input = io::BufReader::new(input_file);
    let input = io::BufReader::new(input_file);
    // println!("3");
    let mut output_file_count = 0;
    // println!("4");
    let mut output_file = None;
    // let buf: &mut String = &mut String::new();
    for line in input.lines() {
        let line = line.unwrap();
        if regex_doc_start().is_match(&line) {
            // Start of a new document, open a new output file.
            output_file = Some(open_new_file_for_output(
                &basename,
                &extension,
                &mut output_file_count,
            )?);
        } else if regex_doc_end().is_match(&line) {
            // End of a document, close the current output file.
            output_file = None;
        } else {
            // Write the line to the output file.
            if output_file.is_none() {
                output_file = Some(open_new_file_for_output(
                    &basename,
                    &extension,
                    &mut output_file_count,
                )?);
            }
            output_line_to_file(&line, &mut output_file)?;
        }
    }
    /*
     */
    Ok(())
}
