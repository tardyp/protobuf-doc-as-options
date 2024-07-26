mod path_resolver;
mod editor;

use std::{io::Write, path::{self, PathBuf}};

use clap::Parser;
use editor::Editor;
use miette::IntoDiagnostic;
use path_resolver::{tag, PathedChilds, PathedDescriptor};
use prost_reflect::{
    prost_types::SourceCodeInfo, DynamicMessage, EnumDescriptor, EnumValueDescriptor, ExtensionDescriptor, FieldDescriptor, FileDescriptor, MessageDescriptor, MethodDescriptor, ServiceDescriptor, Value
};
use protox::Compiler;

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
    output: Option<PathBuf>,
}
fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    entry_point(Args::parse())
}
fn entry_point(args: Args) -> miette::Result<()> {
    let first_include = args
        .includes
        .get(0)
        .expect("at least one include dir is expected")
        .clone();
    let mut compiler = Compiler::new(args.includes)?;
    compiler.include_imports(false);
    compiler.include_source_info(true);
    let out_dir = args.output.or_else(|| Some(PathBuf::from("out"))).unwrap();
    for file_glob in args.files {
        let file_glob = file_glob.to_string_lossy();
        let file_glob = shellexpand::tilde(&file_glob);
        for file in glob::glob(&file_glob).into_diagnostic()? {
            let file = file.into_diagnostic()?;
            let relative = file
                .strip_prefix(&first_include)
                .into_diagnostic()?
                .to_path_buf();
            let out_file = out_dir.join(relative.clone());
            std::fs::create_dir_all(out_file.parent().unwrap()).into_diagnostic()?;
            let in_text = std::fs::read_to_string(&file).into_diagnostic()?;
            let mut out = std::fs::File::create(out_file).into_diagnostic()?;
            compiler.open_file(file)?;
            let fd = compiler
                .descriptor_pool()
                .get_file_by_name(&relative.to_string_lossy())
                .unwrap();
            insert_comments(&fd, &in_text, &mut out)?;
        }
    }
    Ok(())
}
fn insert_comments(
    fd: &FileDescriptor,
    in_text: &str,
    out: &mut std::fs::File,
) -> miette::Result<()> {
    let source_info = fd
        .file_descriptor_proto()
        .source_code_info
        .as_ref()
        .unwrap();
    let mut editor = Editor::new(in_text.to_string());
    for loc in source_info.location.iter() {
        let start_line = loc.span[0] as usize;
        let start_col = loc.span[1] as usize;
        let start = editor.get_position(start_line, start_col);
        if let Some(pathed) = fd.get_child_from_loc(loc) {
            if let Some(ext) = get_description(&pathed) {
                let spaces = &in_text[start - start_col..start];
                let comment = ext.value.as_str().unwrap().to_string();
                let mut to_remove_path = loc.path.clone();
                to_remove_path.push(get_option(&pathed));
                to_remove_path.push(ext.desc.number() as i32);
                let (position, length) = find_to_delete_span(&editor, &source_info, &to_remove_path);
                let (position, length) = match pathed {
                    PathedDescriptor::Field(_) |
                    PathedDescriptor::EnumValue(_) => {
                        let (start, len) = eat_syntax_aroud(&editor, position, length);
                        (start, len)
                    }
                    _ => (position, length),
                };
                editor.delete(position, length);
                editor.insert(start, format_comment(comment, spaces));
            }
        }
    }
    editor.apply();
    out.write_all(editor.text().as_bytes()).into_diagnostic()?;
    Ok(())
}

fn find_to_delete_span(editor: &Editor, source_info: &&SourceCodeInfo, to_remove_path: &[i32]) -> (usize, usize) {
    for loc in source_info.location.iter() {
        if loc.path == *to_remove_path {
            let span = &loc.span;
            let start_line = span[0] as usize;
            let start_col = span[1] as usize;
            let (end_line, end_col) =
                match span.len() {
                    3 => (span[0] as usize, span[2] as usize),
                    4 => (span[2] as usize, span[3] as usize),
                    _ => return (0, 0),
                };
            let mut start = editor.get_position(start_line, start_col);
            let mut end = editor.get_position(end_line, end_col);

            return (start, end - start);
        }
    }
    (0, 0)
}

fn eat_syntax_aroud(editor: &Editor, mut start:usize, len:usize)  -> (usize, usize) {
    let mut end = start + len;
    // eat syntax around (whitespace, commas, etc)
    // and option brackets if they are remaining alone
    while start > 0 {
        match editor.char_at(start - 1) {
            Some('\n') |
            Some(' ') |
            Some(',') |
            Some('\t') =>
                start -= 1,
            None | _ => break,
        
        }
    }
    while end < editor.text().len() {
        match editor.char_at(end) {
            Some('\n') |
            Some(' ') |
            Some('\t') =>
                end += 1,
            None | _ => break,
        
        }
    }
    if editor.char_at(start-1) == Some('[') && editor.char_at(end) == Some(']') {
        start -= 1;
        end += 1;
    }
    (start, end - start)
}
/// Format a comment to fit within 100 characters
/// and add the correct padding
fn format_comment(comment: String, spaces: &str) -> String {
    let mut lines = Vec::new();
    let mut line = String::new();
    let padding_size = spaces.len() + 4;
    for word in comment.split_whitespace() {
        if line.len() + word.len() + padding_size > 100 {
            lines.push(line.clone());
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    lines.push(line);
    let mut formatted = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            formatted.push_str(spaces);
        }
        formatted.push_str("// ");
        formatted.push_str(&line);
        formatted.push('\n');
    }
    formatted.push_str(spaces);
    formatted
}

struct Ext {
    desc: ExtensionDescriptor,
    value: Value,
}

trait Described {
    fn get_description(&self) -> Option<Ext>;
}
impl Described for DynamicMessage {
    fn get_description(&self) -> Option<Ext> {
        self.extensions()
            .find(|ext| ext.0.name().ends_with("description"))
            .map(|(ed, v)| Ext {
                desc: ed,
                value: v.clone(),
            })
    }
}
macro_rules! impl_commented {
    ($($t:ty),*) => {
        $(impl Described for $t {
            fn get_description(&self) -> Option<Ext> {
                self.options().get_description()
            }
        })*
    };
}
impl_commented!(
    MessageDescriptor,
    EnumDescriptor,
    EnumValueDescriptor,
    FieldDescriptor,
    ServiceDescriptor,
    MethodDescriptor
);
fn get_description(pathed: &PathedDescriptor) -> Option<Ext> {
    match pathed {
        PathedDescriptor::Message(m) => m.get_description(),
        PathedDescriptor::Enum(e) => e.get_description(),
        PathedDescriptor::Service(s) => s.get_description(),
        PathedDescriptor::Method(m) => m.get_description(),
        PathedDescriptor::Field(f) => f.get_description(),
        PathedDescriptor::EnumValue(e) => e.get_description(),
        _ => None,
    }
}
fn get_option(pathed: &PathedDescriptor) -> i32 {
    match pathed {
        PathedDescriptor::Message(_) => tag::message::OPTIONS,
        PathedDescriptor::Enum(_) => tag::enum_::OPTIONS,
        PathedDescriptor::Service(_) => tag::service::OPTIONS,
        PathedDescriptor::Method(_) => tag::method::OPTIONS,
        PathedDescriptor::Field(_) => tag::field::OPTIONS,
        PathedDescriptor::EnumValue(_) => tag::enum_value::OPTIONS,
        _ => 0,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand;
    #[test]
    fn test_format_comment() {
        let comment = "This is a long comment that should be split into multiple lines to fit within 100 characters".to_string();
        let spaces = "    ";
        let formatted = format_comment(comment, spaces);
        assert_eq!(formatted.lines().count(), 2);
    }

    fn run_fixture_test(fixture: &str) {
        let mut fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        fixtures.push("src/fixtures");
        let path = fixtures.join(fixture);
        let temp_output_dir = std::env::temp_dir().join(rand::random::<u64>().to_string());
        let args = Args {
            files: vec![path],
            includes: vec![fixtures.clone()],
            output: Some(temp_output_dir.clone()),
        };
        entry_point(args).unwrap();
        let expected_path = fixtures.join(fixture).with_extension("expected.proto");
        let actual = std::fs::read_to_string(temp_output_dir.join(fixture)).unwrap();
        std::fs::remove_file(expected_path.clone()).unwrap();
        if expected_path.exists() {
            let expected = std::fs::read_to_string(expected_path).unwrap();
            assert_eq!(expected, actual);
        } else {
            // if it is not present, write expected file from generation
            std::fs::write(expected_path, actual).unwrap();
        }
        std::fs::remove_dir_all(temp_output_dir).unwrap();
    }
    #[test]
    fn test_basic() {
        run_fixture_test("basic.proto");
    }
    #[test]
    fn test_nested() {
        run_fixture_test("nested.proto");
    }
}
