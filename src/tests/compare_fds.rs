use protobuf::{self, Message};
use protobuf::text_format::print_to_string_pretty;
use pretty_assertions::assert_eq;

pub(crate) fn compare_fds(expected: &[u8], res: &[u8], fixture: &str) {
    let expected = protobuf::descriptor::FileDescriptorSet::parse_from_bytes(expected).unwrap();
    let mut res = protobuf::descriptor::FileDescriptorSet::parse_from_bytes(res).unwrap();
    for (i, file) in res.file.iter_mut().enumerate() {
        file.source_code_info.clear();
        let expected_file = &expected.file[i];
        file.name = expected_file.name.clone();
        // print to bptxt
        let f1 = print_to_string_pretty(file);
        let f2 = print_to_string_pretty(&expected.file[i]);
        assert_eq!(f1, f2, "Failed test: {}", fixture);
    }
}