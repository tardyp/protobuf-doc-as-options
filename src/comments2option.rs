use crate::path_resolver::protobuf::{PathedChilds, PathedDescriptor};
use miette::IntoDiagnostic;
use protobuf::{descriptor::FileDescriptorSet, Message};

#[derive(Debug, Default)]
pub struct DescriptionIds {
    pub file: Option<u32>,
    pub message: Option<u32>,
    pub enum_: Option<u32>,
    pub service: Option<u32>,
    pub method: Option<u32>,
    pub field: Option<u32>,
    pub enum_value: Option<u32>,
    pub extension: Option<u32>,
    pub oneof: Option<u32>,
}
pub fn comments2option(res: &[u8], ids: &DescriptionIds) -> miette::Result<Vec<u8>> {
    let mut res = FileDescriptorSet::parse_from_bytes(res).into_diagnostic()?;
    for file in &mut res.file {
        if file.name().starts_with("google") {
            continue;
        }
        let sci = file.source_code_info.clone();
        for loc in sci.location.iter() {
            let comments = if loc.has_leading_comments() {
                loc.leading_comments.as_ref().unwrap().clone()
            } else if loc.has_trailing_comments() {
                loc.trailing_comments.as_ref().unwrap().clone()
            } else {
                continue;
            };
            let comments = comments.trim().to_string();
            match file.get_child_from_loc(loc) {
                Some(pathed) => {
                    insert_comment(pathed, comments, ids);
                }
                None => {}
            }
        }
    }
    Ok(res.write_to_bytes().into_diagnostic()?)
}
macro_rules! insert_comment {
    ($x: ident, $comment: ident, $id: expr) => {
        if let Some(id) = $id {
            $x.options
                .mut_or_insert_default()
                .special_fields
                .mut_unknown_fields()
                .add_length_delimited(id, $comment);
        }
    };
}
fn insert_comment(pathed: PathedDescriptor, comment: String, ids: &DescriptionIds) {
    let comment = comment.as_bytes().to_vec();
    match pathed {
        PathedDescriptor::Message(message) => {
            insert_comment!(message, comment, ids.message);
        }
        PathedDescriptor::Enum(enum_) => {
            insert_comment!(enum_, comment, ids.enum_);
        }
        PathedDescriptor::Service(service) => {
            insert_comment!(service, comment, ids.service);
        }
        PathedDescriptor::Method(method) => {
            insert_comment!(method, comment, ids.method);
        }
        PathedDescriptor::Field(field) => {
            insert_comment!(field, comment, ids.field);
        }
        PathedDescriptor::EnumValue(enum_value) => {
            insert_comment!(enum_value, comment, ids.enum_value);
        }
        PathedDescriptor::Extension(extension) => {
            insert_comment!(extension, comment, ids.extension);
        }
        PathedDescriptor::Oneof(oneof) => {
            insert_comment!(oneof, comment, ids.oneof);
        }
    }
}


#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use protox::Compiler;
    use crate::tests::compare_fds::compare_fds;

    fn comments2option_test(fixture: &str) {
        let ids = DescriptionIds {
            file: Some(1000),
            message: Some(1000),
            enum_: Some(1000),
            service: Some(1000),
            method: Some(1000),
            field: Some(1000),
            enum_value: Some(1000),
            extension: Some(1000),
            oneof: Some(1000),
        };
        let mut fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        fixtures.push("src/fixtures");
        let path = fixtures.join(fixture).with_extension("expected.proto");
        let mut c = Compiler::new(vec![fixtures.clone()]).unwrap();
        c.include_imports(true);
        c.include_source_info(true);
        c.open_file(path).unwrap();
        let v = c.encode_file_descriptor_set();
        let res = comments2option(&v, &ids).unwrap();
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
    #[test]
    fn test_nested() {
        comments2option_test("nested.proto");
    }
    #[test]
    fn test_siblings() {
        comments2option_test("siblings.proto");
    }
}