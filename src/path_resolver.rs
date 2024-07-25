use prost_reflect::{
    prost_types::source_code_info, DynamicMessage, EnumDescriptor, EnumValueDescriptor, ExtensionDescriptor, FieldDescriptor, FileDescriptor, MessageDescriptor, MethodDescriptor, OneofDescriptor, ServiceDescriptor, Value
};

use std::collections::VecDeque;
#[allow(dead_code)]
mod tag;
pub (crate)enum PathedDescriptor {
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
pub(crate) fn path_resolver<'a>(fd: &'a FileDescriptor, loc: &source_code_info::Location) -> Option<PathedDescriptor> {
    let mut path: VecDeque<i32> = loc.path.iter().copied().collect();
    // pop first element
    let typ = path.pop_front()?;
    let idx = path.pop_front()? as usize;
    match typ {
        tag::file::MESSAGE_TYPE => {
            let message = fd.messages().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::Message(message))
            } else {
                get_pathed_descriptor_for_message(message, path)
            }
        }
        tag::file::ENUM_TYPE => {
            let enum_ = fd.enums().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::Enum(enum_))
            } else {
                get_pathed_descriptor_for_enum(enum_, path)
            }
        }
        tag::file::SERVICE => {
            let service = fd.services().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::Service(service))
            } else {
                get_pathed_descriptor_for_service(service, path)
            }
        }
        tag::file::EXTENSION => {
            let extension = fd.extensions().nth(idx)?;
            Some(PathedDescriptor::Extension(extension))
        }
        tag::file::OPTIONS => {
            get_option_field(fd.options(), idx)
        }
        _ => None,
    }
}

fn get_pathed_descriptor_for_service(service: ServiceDescriptor, mut path: VecDeque<i32>) -> Option<PathedDescriptor> {
    let typ = path.pop_front()?;
    let idx = path.pop_front()? as usize;
    match typ {
        tag::service::METHOD => {
            let method = service.methods().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::Method(method))
            } else {
                None
            }
        }
        tag::service::OPTIONS => get_option_field(service.options(), idx),
        _ => None,
    }
}

fn get_pathed_descriptor_for_enum(enum_: EnumDescriptor, mut path: VecDeque<i32>) -> Option<PathedDescriptor> {
    let typ = path.pop_front()?;
    let idx = path.pop_front()? as usize;
    match typ {
        tag::enum_::VALUE => {
            let value = enum_.values().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::EnumValue(value))
            } else {
                None
            }
        }
        tag::enum_::OPTIONS => get_option_field(enum_.options(), idx),
        tag::enum_::RESERVED_RANGE => {
            let reserved_range = enum_.reserved_ranges().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::EnumReservedRange(reserved_range))
            } else {
                None
            }
        }
        _ => None,
    }

}

fn get_pathed_descriptor_for_message(message: MessageDescriptor, mut path: VecDeque<i32>) -> Option<PathedDescriptor> {
    let typ = path.pop_front()?;
    let idx = path.pop_front()? as usize;
    match typ {
        tag::message::FIELD => {
            let field = message.fields().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::Field(field))
            } else {
                None
            }
        }
        tag::message::ENUM_TYPE => {
            let enum_ = message.child_enums().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::Enum(enum_))
            } else {
                get_pathed_descriptor_for_enum(enum_, path)
            }
        }
        tag::message::EXTENSION_RANGE => {
            let extension_range = message.extension_ranges().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::ExtensionRange(extension_range))
            } else {
                None
            }
        }
        tag::message::NESTED_TYPE => {
            let nested_type = message.child_messages().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::Message(nested_type))
            } else {
                get_pathed_descriptor_for_message(nested_type, path)
            }
        }
        tag::message::OPTIONS => get_option_field(message.options(), idx),
        tag::message::ONEOF_DECL => {
            let oneof = message.oneofs().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::Oneof(oneof))
            } else {
                None
            }
        }
        tag::message::RESERVED_RANGE => {
            let reserved_range = message.reserved_ranges().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::ReservedRange(reserved_range))
            } else {
                None
            }
        }
        tag::message::RESERVED_NAME => {
            let reserved_name = message.reserved_names().nth(idx)?;
            if path.is_empty() {
                Some(PathedDescriptor::ReservedName(reserved_name.to_string()))
            } else {
                None
            }
        }
        _ => None,
    }
}