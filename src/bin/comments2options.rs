
use clap::Parser;
use miette::IntoDiagnostic;
use protox::Compiler;
use std::{io::Write, path::PathBuf};
use protox_doc::comments2option::{comments2option, DescriptionIds};

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
    // options for DescriptionIds
    /// The ID of the option for FileDescriptor comments
    #[clap(long = "file-id", value_name = "ID")]
    file: Option<u32>,
    /// The ID of the option for Message comments
    #[clap(long = "message-id", value_name = "ID")]
    message: Option<u32>,
    /// The ID of the option for Enum comments
    #[clap(long = "enum-id", value_name = "ID")]
    enum_: Option<u32>,
    /// The ID of the option for Service comments
    #[clap(long = "service-id", value_name = "ID")]
    service: Option<u32>,
    /// The ID of the option for Method comments
    #[clap(long = "method-id", value_name = "ID")]
    method: Option<u32>,
    /// The ID of the option for Field comments
    #[clap(long = "field-id", value_name = "ID")]
    field: Option<u32>,
    /// The ID of the option for EnumValue comments
    #[clap(long = "enum-value-id", value_name = "ID")]
    enum_value: Option<u32>,
    /// The ID of the option for Extension comments
    #[clap(long = "extension-id", value_name = "ID")]
    extension: Option<u32>,
    /// The ID of the option for Oneof comments
    #[clap(long = "oneof-id", value_name = "ID")]
    oneof: Option<u32>,    
}
fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    entry_point(Args::parse())
}
fn entry_point(args: Args) -> miette::Result<()> {
    let mut compiler = Compiler::new(args.includes)?;
    let ids = DescriptionIds {
        file: args.file,
        message: args.message,
        enum_: args.enum_,
        service: args.service,
        method: args.method,
        field: args.field,
        enum_value: args.enum_value,
        extension: args.extension,
        oneof: args.oneof,
    };
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
    let res = comments2option(&res, &ids);
    std::fs::File::create(&args.output).into_diagnostic()?.write_all(&res).into_diagnostic()?;
    Ok(())
}
