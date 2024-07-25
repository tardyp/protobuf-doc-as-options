use prost_reflect::{
    prost_types::source_code_info, DynamicMessage, EnumDescriptor, EnumValueDescriptor,
    ExtensionDescriptor, FieldDescriptor, FileDescriptor, MessageDescriptor, MethodDescriptor,
    OneofDescriptor, ServiceDescriptor, Value,
};

use std::collections::VecDeque;
#[allow(dead_code)]
pub(crate)mod tag;
pub(crate) enum PathedDescriptor {
    Message(MessageDescriptor),
    Enum(EnumDescriptor),
    Service(ServiceDescriptor),
    Method(MethodDescriptor),
    Field(FieldDescriptor),
    EnumValue(EnumValueDescriptor),
    Option((FieldDescriptor, Value)),
    Extension(ExtensionDescriptor),
    EnumReservedRange(std::ops::RangeInclusive<i32>),
    ReservedRange(std::ops::Range<u32>),
    ExtensionRange(std::ops::Range<u32>),
    Oneof(OneofDescriptor),
    ReservedName(String),
}
fn get_option_field(opt: DynamicMessage, idx: usize) -> Option<PathedDescriptor> {
    let (fd, value) = opt.fields().nth(idx)?;
    Some(PathedDescriptor::Option((fd, value.clone())))
}
pub(crate) trait PathedChilds {
    fn get_child_from_path(&self, path: &mut VecDeque<i32>) -> Option<PathedDescriptor>;
    fn get_child_from_loc(&self, loc: &source_code_info::Location) -> Option<PathedDescriptor> {
        let mut path: VecDeque<i32> = loc.path.iter().copied().collect();
        self.get_child_from_path(&mut path)
    }
}
impl PathedChilds for FileDescriptor {
    fn get_child_from_path(&self, path: &mut VecDeque<i32>) -> Option<PathedDescriptor> {
        // pop first element
        let typ = path.pop_front()?;
        let idx = path.pop_front()? as usize;
        match typ {
            tag::file::MESSAGE_TYPE => {
                let message = self.messages().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Message(message))
                } else {
                    message.get_child_from_path(path)
                }
            }
            tag::file::ENUM_TYPE => {
                let enum_ = self.enums().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Enum(enum_))
                } else {
                    enum_.get_child_from_path(path)
                }
            }
            tag::file::SERVICE => {
                let service = self.services().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Service(service))
                } else {
                    service.get_child_from_path(path)
                }
            }
            tag::file::EXTENSION => {
                let extension = self.extensions().nth(idx)?;
                Some(PathedDescriptor::Extension(extension))
            }
            tag::file::OPTIONS => get_option_field(self.options(), idx),
            _ => None,
        }
    }
}

impl PathedChilds for ServiceDescriptor {
    fn get_child_from_path(&self, path: &mut VecDeque<i32>) -> Option<PathedDescriptor> {
        let typ = path.pop_front()?;
        let idx = path.pop_front()? as usize;
        match typ {
            tag::service::METHOD => {
                let method = self.methods().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Method(method))
                } else {
                    None
                }
            }
            tag::service::OPTIONS => get_option_field(self.options(), idx),
            _ => None,
        }
    }
}
impl PathedChilds for EnumDescriptor {
    fn get_child_from_path(&self, path: &mut VecDeque<i32>) -> Option<PathedDescriptor> {
        let typ = path.pop_front()?;
        let idx = path.pop_front()? as usize;
        match typ {
            tag::enum_::VALUE => {
                let value = self.values().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::EnumValue(value))
                } else {
                    None
                }
            }
            tag::enum_::OPTIONS => get_option_field(self.options(), idx),
            tag::enum_::RESERVED_RANGE => {
                let reserved_range = self.reserved_ranges().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::EnumReservedRange(reserved_range))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl PathedChilds for MessageDescriptor {
    fn get_child_from_path(&self, path: &mut VecDeque<i32>) -> Option<PathedDescriptor> {
        let typ = path.pop_front()?;
        let idx = path.pop_front()? as usize;
        match typ {
            tag::message::FIELD => {
                let field = self.fields().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Field(field))
                } else {
                    None
                }
            }
            tag::message::ENUM_TYPE => {
                let enum_ = self.child_enums().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Enum(enum_))
                } else {
                    enum_.get_child_from_path(path)
                }
            }
            tag::message::EXTENSION_RANGE => {
                let extension_range = self.extension_ranges().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::ExtensionRange(extension_range))
                } else {
                    None
                }
            }
            tag::message::NESTED_TYPE => {
                let nested_type = self.child_messages().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Message(nested_type))
                } else {
                    nested_type.get_child_from_path(path)
                }
            }
            tag::message::OPTIONS => get_option_field(self.options(), idx),
            tag::message::ONEOF_DECL => {
                let oneof = self.oneofs().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Oneof(oneof))
                } else {
                    None
                }
            }
            tag::message::RESERVED_RANGE => {
                let reserved_range = self.reserved_ranges().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::ReservedRange(reserved_range))
                } else {
                    None
                }
            }
            tag::message::RESERVED_NAME => {
                let reserved_name = self.reserved_names().nth(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::ReservedName(reserved_name.to_string()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
