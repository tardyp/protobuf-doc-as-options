
use clap::Parser;
use miette::IntoDiagnostic;
use protox::Compiler;
use std::{io::Write, path::PathBuf};
mod comments2option;
use comments2option::comments2option;

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// The source file(s) to compile
    #[clap(value_name = "PROTO_FILES", required = true, value_parser)]
    files: Vec<PathBuf>,
    /// The directory in which to search for imports.
    #[clap(
        short = 'I',
        long = "include",
        visible_alias = "proto_path_glob",
        value_name = "PATH_GLOB",
        default_value = ".",
        value_parser
    )]
    includes: Vec<PathBuf>,
    /// The output path to write the modified files to.
    #[clap(
        short = 'o',
        long = "output",
        visible_alias = "output_dir",
        value_name = "PATH",
        value_parser
    )]
    output: PathBuf,
}
fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    entry_point(Args::parse())
}
fn entry_point(args: Args) -> miette::Result<()> {
    let mut compiler = Compiler::new(args.includes)?;
    compiler.include_imports(true);
    compiler.include_source_info(true);
    for file_glob in args.files {
        let file_glob = file_glob.to_string_lossy();
        let file_glob = shellexpand::tilde(&file_glob);
        for file in glob::glob(&file_glob).into_diagnostic()? {
            let file = file.into_diagnostic()?;
            compiler.open_file(file)?;
        }
    }
    let res = compiler.encode_file_descriptor_set();
    let res = comments2option(&res)?;
    std::fs::File::create(&args.output).into_diagnostic()?.write_all(&res).into_diagnostic()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    mod compare_fds;
    use compare_fds::compare_fds;
    fn comments2option_test(fixture: &str) {
        let mut fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        fixtures.push("src/fixtures");
        let path = fixtures.join(fixture).with_extension("expected.proto");
        let mut c = Compiler::new(vec![fixtures.clone()]).unwrap();
        c.include_imports(true);
        c.include_source_info(true);
        c.open_file(path).unwrap();
        let v = c.encode_file_descriptor_set();
        let res = comments2option(&v).unwrap();
        let path = fixtures.join(fixture);
        let mut c = Compiler::new(vec![fixtures.clone()]).unwrap();
        c.include_imports(true);
        c.include_source_info(false);
        c.open_file(path).unwrap();
        let expected = c.encode_file_descriptor_set();
        compare_fds(&expected, &res, fixture);
    }
    #[test]
    fn test_basic() {
        comments2option_test("basic.proto");
    }
}