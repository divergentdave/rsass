use deunicode::deunicode;
use hrx_get::Archive;
use std::ffi::OsStr;
use std::fs::{create_dir, DirEntry, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

mod options;
use options::Options;
mod testfixture;
use testfixture::TestFixture;

fn main() -> Result<(), Error> {
    let base = PathBuf::from("sass-spec");
    handle_suite(
        &base,
        "spec",
        &[
            "core_functions/selector/extend", // not supported
            "core_functions/selector/is_superselector", // not supported
            "core_functions/selector/unify",  // not supported
            "directives/forward", // `@forward` is not supported at all
            "directives/use",     // `@use` is not supported at all
            "libsass-closed-issues/issue_185/mixin.hrx", // stack overflow
            "libsass-todo-issues/issue_221262.hrx", // stack overflow
            "libsass-todo-issues/issue_221292.hrx", // stack overflow
            "libsass/Sa\u{301}ss-UT\u{327}F8.hrx", // duplicate rust name
            "libsass/unicode-bom/utf-16-big", // rsass only handles utf8
            "libsass/unicode-bom/utf-16-little", // rsass only handles utf8
            "non_conformant/scss/huge.hrx", // stack overflow in debug mode
            "non_conformant/scss/mixin-content.hrx", // stack overflow
            "non_conformant/scss/multiline_var.hrx", // duplicate rust name
        ],
    )?;
    Ok(())
}

fn handle_suite(
    base: &Path,
    suite: &str,
    ignored: &[&str],
) -> Result<(), Error> {
    eprintln!("Handle suite {:?}", suite);
    let suitedir = base.join(suite);
    let rssuitedir = PathBuf::from("tests").join(fn_name(suite));
    let _may_exist = create_dir(&rssuitedir);
    let mut rs = File::create(rssuitedir.join("main.rs"))?;
    writeln!(
        rs,
        "//! Tests auto-converted from {:?}\
         \n//! version {}\
         \n//! See <https://github.com/sass/sass-spec> for source material.\\n",
        suitedir,
        String::from_utf8(
            Command::new("git")
                .args(&["log", "-1", "--format=%h, %ai."])
                .current_dir(base)
                .output()?
                .stdout
        )?.trim(),
    )?;
    if !ignored.is_empty() {
        writeln!(
            rs,
            "//! The following tests are excluded from conversion:\
             \n//! {:?}",
            ignored,
        )?;
    }
    writeln!(rs, "use rsass::{{compile_scss, output::Format}};",)?;

    handle_entries(&mut rs, &base, &suitedir, &rssuitedir, None, ignored)
        .map_err(|e| {
            Error(format!("Failed to handle suite {:?}: {}", suite, e))
        })?;

    writeln!(
        rs,
        "\nfn rsass(input: &str) -> Result<String, String> {{\
         \n    compile_scss(input.as_bytes(), Default::default())\
         \n        .map_err(|e| format!(\"rsass failed: {{}}\", e))\
         \n        .and_then(|s| {{\
         \n            String::from_utf8(s)\
         \n                .map(|s| s.replace(\"\\n\\n\", \"\\n\"))\
         \n                .map_err(|e| format!(\"{{:?}}\", e))\
         \n        }})\
         \n}}\
         \n#[allow(unused)]\
         \nfn rsass_fmt(format: Format, input: &str)\
         \n-> Result<String, String> {{\
         \n    compile_scss(input.as_bytes(), format)\
         \n        .map_err(|e| format!(\"rsass failed: {{}}\", e))\
         \n        .and_then(|s| {{\
         \n            String::from_utf8(s)\
         \n                .map(|s| s.replace(\"\\n\\n\", \"\\n\"))\
         \n                .map_err(|e| format!(\"{{:?}}\", e))\
         \n        }})\
         \n}}"
    )?;
    Ok(())
}

fn handle_entries(
    rs: &mut dyn Write,
    root: &Path,
    suitedir: &Path,
    rssuitedir: &Path,
    precision: Option<i64>,
    ignored: &[&str],
) -> Result<(), Error> {
    let mut entries: Vec<DirEntry> =
        suitedir.read_dir()?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        if ignored.iter().any(|&i| &entry.file_name() == i) {
            ignore(rs, &entry.file_name(), "not expected to work yet")?;
        } else if entry.file_type()?.is_file()
            && entry.file_name().to_string_lossy().ends_with(".hrx")
        {
            spec_hrx_to_test(rs, &entry.path(), precision).map_err(|e| {
                Error(format!("Failed to handle {:?}: {}", entry.path(), e))
            })?;
        } else if entry.file_type()?.is_dir() {
            if entry.path().join("error").is_file() {
                ignore(
                    rs,
                    &entry.file_name(),
                    "tests with expected error not implemented yet",
                )?;
            } else {
                eprintln!(
                    "Should handle {}",
                    entry.path().strip_prefix(root)?.display()
                );
                let input = entry.path().join("input.scss");
                if input.exists() {
                    spec_dir_to_test(
                        rs,
                        &suitedir,
                        &entry.file_name(),
                        precision,
                    )
                    .map_err(|e| {
                        Error(format!(
                            "Failed to handle {:?}: {}",
                            entry.file_name(),
                            e,
                        ))
                    })?;
                } else {
                    let options = load_options(&entry.path())?;
                    if let Some(ref reason) = options.should_skip {
                        ignore(rs, &entry.file_name(), reason)?;
                    } else {
                        let precision = options.precision.or(precision);
                        let name = fn_name_os(&entry.file_name());
                        writeln!(rs, "\nmod {};", name)?;
                        let rssuitedir = rssuitedir.join(name);
                        let _may_exist = create_dir(&rssuitedir);
                        let mut rs = File::create(rssuitedir.join("mod.rs"))?;
                        writeln!(
                            rs,
                            "//! Tests auto-converted from {:?}\
                             \n#[allow(unused)]\
                             \nuse super::rsass;",
                            suitedir.join(entry.file_name()),
                        )?;
                        let tt = format!(
                            "{}/",
                            entry.file_name().to_string_lossy(),
                        );
                        let ignored: Vec<&str> = ignored
                            .iter()
                            .filter_map(|p| {
                                if p.starts_with(&tt) {
                                    Some(p.split_at(tt.len()).1)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        handle_entries(
                            &mut rs,
                            &root,
                            &entry.path(),
                            &rssuitedir,
                            precision,
                            &ignored,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn ignore<T: std::fmt::Debug>(
    rs: &mut dyn Write,
    name: &T,
    reason: &str,
) -> Result<(), io::Error> {
    eprintln!("Ignoring {:?}, {}.", name, reason);
    writeln!(rs, "\n// Ignoring {:?}, {}.", name, reason)
}

fn spec_dir_to_test(
    rs: &mut dyn Write,
    suite: &Path,
    test: &OsStr,
    precision: Option<i64>,
) -> Result<(), Error> {
    let specdir = suite.join(test);
    let fixture = load_test_fixture_dir(&specdir, precision)?;
    writeln!(rs, "\n// From {:?}", specdir)?;
    fixture.write_test(rs)
}

fn spec_hrx_to_test(
    rs: &mut dyn Write,
    suite: &Path,
    precision: Option<i64>,
) -> Result<(), Error> {
    let archive = Archive::load(suite)
        .map_err(|e| Error(format!("Failed to load hrx: {}", e)))?;

    writeln!(rs, "\n// From {:?}", suite)?;
    eprintln!("Handle {}", suite.display());
    handle_hrx_part(rs, suite, &archive, "", precision)
}

fn handle_hrx_part(
    rs: &mut dyn Write,
    suite: &Path,
    archive: &Archive,
    prefix: &str,
    precision: Option<i64>,
) -> Result<(), Error> {
    use std::collections::BTreeSet;
    let tests = archive
        .names()
        .iter()
        .flat_map(|s| {
            if s.starts_with(prefix) {
                let t = prefix.len();
                if let Some(pos) = s[t..].find('/') {
                    Some(&s[t..t + pos + 1])
                } else {
                    None // Some("")
                }
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();

    let name = if prefix.is_empty() {
        fn_name_os(&suite.file_name().unwrap_or_default())
    } else {
        let t = if prefix.ends_with('/') {
            &prefix[0..prefix.len() - 1]
        } else {
            prefix
        };
        let t = if let Some(p) = t.rfind('/') {
            &t[p + 1..]
        } else {
            t
        };
        fn_name(t)
    };

    if archive.get(&format!("{}input.scss", prefix)).is_some() {
        let fixture =
            load_test_fixture_hrx(name, &archive, prefix, precision)?;
        fixture.write_test(rs)
    } else {
        let options = archive
            .get(&format!("{}options.yml", prefix))
            .map(Options::parse)
            .transpose()?
            .unwrap_or_default();
        if let Some(ref reason) = options.should_skip {
            ignore(rs, &suite.file_name(), reason).map_err(|e| e.into())
        } else {
            let precision = options.precision.or(precision);
            if !tests.is_empty() {
                writeln!(
                    rs,
                    "mod {} {{\n#[allow(unused)]\nuse super::rsass;",
                    name,
                )?;
                for name in tests {
                    handle_hrx_part(
                        rs,
                        suite,
                        &archive,
                        &format!("{}{}", prefix, name),
                        precision,
                    )?;
                }
                writeln!(rs, "}}")?;
            }
            Ok(())
        }
    }
}

fn fn_name_os(name: &OsStr) -> String {
    fn_name(&name.to_string_lossy())
}
fn fn_name(name: &str) -> String {
    let t = deunicode(name)
        .to_lowercase()
        .replace(".hrx", "")
        .replace('/', "_")
        .replace('-', "_")
        .replace('.', "_");
    if t.chars().next().unwrap_or('0').is_numeric() {
        format!("t{}", t)
    } else if t == "as"
        || t == "else"
        || t == "false"
        || t == "final"
        || t == "for"
        || t == "if"
        || t == "loop"
        || t == "match"
        || t == "override"
        || t == "static"
        || t == "super"
        || t == "true"
        || t == "type"
        || t == "use"
        || t == "while"
    {
        format!("test_{}", t)
    } else {
        t
    }
}

fn load_test_fixture_dir(
    specdir: &Path,
    precision: Option<i64>,
) -> Result<TestFixture, Error> {
    static INPUT_FILENAME: &str = "input.scss";
    static EXPECTED_OUTPUT_FILENAME: &str = "output.css";
    static EXPECTED_ERROR_FILENAMES: &[&str] = &["error-dart-sass", "error"];

    let name = fn_name_os(specdir.file_name().unwrap_or_default());
    let mut options = load_options(&specdir)?;
    options.precision = options.precision.or(precision);
    let input = content(&specdir.join(INPUT_FILENAME))?;

    {
        let path = specdir.join(EXPECTED_OUTPUT_FILENAME);
        if path.exists() {
            return Ok(TestFixture::new_ok(
                name,
                input,
                &content(&path)?,
                options,
            ));
        }
    }
    for filename in EXPECTED_ERROR_FILENAMES {
        let path = specdir.join(filename);
        if path.exists() {
            return Ok(TestFixture::new_err(
                name,
                input,
                content(&path)?,
                options,
            ));
        }
    }
    Err(Error(format!(
        "No expected CSS / error found for {}",
        specdir.file_name().unwrap_or_default().to_str().unwrap()
    )))
}

fn load_test_fixture_hrx(
    name: String,
    archive: &Archive,
    prefix: &str,
    precision: Option<i64>,
) -> Result<TestFixture, Error> {
    static INPUT_FILENAME: &str = "input.scss";
    static EXPECTED_OUTPUT_FILENAMES: &[&str] =
        &["output-dart-sass.css", "output.css", "output-libsass.css"];
    static EXPECTED_ERROR_FILENAMES: &[&str] =
        &["error-dart-sass", "error", "error-libsass"];

    let mut options =
        if let Some(yml) = archive.get(&format!("{}options.yml", prefix)) {
            Options::parse(yml)?
        } else {
            Options::default()
        };
    options.precision = options.precision.or(precision);

    if let Some(input) = archive.get(&format!("{}{}", prefix, INPUT_FILENAME))
    {
        for filename in EXPECTED_OUTPUT_FILENAMES {
            if let Some(output) =
                archive.get(&format!("{}{}", prefix, filename))
            {
                return Ok(TestFixture::new_ok(
                    name,
                    input.to_string(),
                    output,
                    options,
                ));
            }
        }
        for filename in EXPECTED_ERROR_FILENAMES {
            if let Some(error) =
                archive.get(&format!("{}{}", prefix, filename))
            {
                return Ok(TestFixture::new_err(
                    name,
                    input.to_string(),
                    error.to_string(),
                    options,
                ));
            }
        }
        return Err(Error(format!(
            "No expected CSS / error found for {:?}",
            prefix,
        )));
    }
    Err(Error(format!("No input found for {:?}", prefix)))
}

/// Load options from options.yml.
fn load_options(path: &Path) -> Result<Options, Error> {
    let yml = path.join("options.yml");
    if yml.exists() {
        Options::parse(&content(&yml)?)
    } else {
        Ok(Options {
            precision: None,
            should_skip: None,
        })
    }
}

fn content(path: &Path) -> Result<String, io::Error> {
    let mut buf = String::new();
    File::open(path)?.read_to_string(&mut buf)?;
    Ok(buf)
}

#[derive(Debug)]
pub struct Error(String);
use std::convert::From;

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error(format!("io error: {}", e))
    }
}
impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Error(format!("utf8 error: {}", e))
    }
}
impl From<yaml_rust::ScanError> for Error {
    fn from(e: yaml_rust::ScanError) -> Self {
        Error(format!("utf8 error: {}", e))
    }
}
impl From<std::num::ParseFloatError> for Error {
    fn from(e: std::num::ParseFloatError) -> Self {
        Error(format!("utf8 error: {}", e))
    }
}
impl From<std::path::StripPrefixError> for Error {
    fn from(e: std::path::StripPrefixError) -> Self {
        Error(format!("{}", e))
    }
}

use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(out)
    }
}
