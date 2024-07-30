use protobuf::descriptor::{
        source_code_info::Location, DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto, FileDescriptorProto, MethodDescriptorProto, OneofDescriptorProto, ServiceDescriptorProto
    };
use std::collections::VecDeque;
use super::tag;


pub(crate) enum PathedDescriptor {
    Message(DescriptorProto),
    Enum(EnumDescriptorProto),
    Service(ServiceDescriptorProto),
    Method(MethodDescriptorProto),
    Field(FieldDescriptorProto),
    EnumValue(EnumValueDescriptorProto),
    // Option((FieldDescriptorProto, protobuf::Value)),
    Extension(FieldDescriptorProto), // Assuming extensions are represented as FieldDescriptorProto
    // EnumReservedRange(std::ops::RangeInclusive<i32>),
    // ReservedRange(std::ops::Range<u32>),
    // ExtensionRange(std::ops::Range<u32>),
    Oneof(OneofDescriptorProto),
    ReservedName(String),
}


pub(crate) trait PathedChilds {
    fn get_child_from_path(&self, path: &mut VecDeque<i32>) -> Option<PathedDescriptor>;
    fn get_child_from_loc(&self, loc: &Location) -> Option<PathedDescriptor> {
        let mut path: VecDeque<i32> = loc.path.iter().copied().collect();
        self.get_child_from_path(&mut path)
    }
}

impl PathedChilds for FileDescriptorProto {
    fn get_child_from_path(&self, path: &mut VecDeque<i32>) -> Option<PathedDescriptor> {
        let typ = path.pop_front()?;
        let idx = path.pop_front()? as usize;
        match typ {
            tag::file::MESSAGE_TYPE => {
                let message = self.message_type.get(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Message(message.clone()))
                } else {
                    message.get_child_from_path(path)
                }
            }
            tag::file::ENUM_TYPE => {
                let enum_ = self.enum_type.get(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Enum(enum_.clone()))
                } else {
                    enum_.get_child_from_path(path)
                }
            }
            tag::file::SERVICE => {
                let service = self.service.get(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Service(service.clone()))
                } else {
                    service.get_child_from_path(path)
                }
            }
            tag::file::EXTENSION => {
                let extension = self.extension.get(idx)?;
                Some(PathedDescriptor::Extension(extension.clone()))
            }
            _ => None,
        }
    }
}

impl PathedChilds for ServiceDescriptorProto {
    fn get_child_from_path(&self, path: &mut VecDeque<i32>) -> Option<PathedDescriptor> {
        let typ = path.pop_front()?;
        let idx = path.pop_front()? as usize;
        match typ {
            tag::service::METHOD => {
                let method = self.method.get(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Method(method.clone()))
                } else {
                    None
                }
            }
            // tag::service::OPTIONS => get_option_field(self.options.as_ref()?, idx),
            _ => None,
        }
    }
}

impl PathedChilds for EnumDescriptorProto {
    fn get_child_from_path(&self, path: &mut VecDeque<i32>) -> Option<PathedDescriptor> {
        let typ = path.pop_front()?;
        let idx = path.pop_front()? as usize;
        match typ {
            tag::enum_::VALUE => {
                let value = self.value.get(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::EnumValue(value.clone()))
                } else {
                    None
                }
            }
            // tag::enum_::OPTIONS => get_option_field(self.options.as_ref()?, idx),
            // tag::enum_::RESERVED_RANGE => {
            //     let reserved_range = self.reserved_range.get(idx)?;
            //     if path.is_empty() {
            //         Some(PathedDescriptor::EnumReservedRange(reserved_range.clone()))
            //     } else {
            //         None
            //     }
            // }
            _ => None,
        }
    }
}

impl PathedChilds for DescriptorProto {
    fn get_child_from_path(&self, path: &mut VecDeque<i32>) -> Option<PathedDescriptor> {
        let typ = path.pop_front()?;
        let idx = path.pop_front()? as usize;
        match typ {
            tag::message::FIELD => {
                let field = self.field.get(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Field(field.clone()))
                } else {
                    None
                }
            }
            tag::message::ENUM_TYPE => {
                let enum_ = self.enum_type.get(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Enum(enum_.clone()))
                } else {
                    enum_.get_child_from_path(path)
                }
            }
            // tag::message::EXTENSION_RANGE => {
            //     let extension_range = self.extension_range.get(idx)?;
            //     if path.is_empty() {
            //         Some(PathedDescriptor::ExtensionRange(extension_range.clone()))
            //     } else {
            //         None
            //     }
            // }
            tag::message::NESTED_TYPE => {
                let nested_type = self.nested_type.get(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Message(nested_type.clone()))
                } else {
                    nested_type.get_child_from_path(path)
                }
            }
            // tag::message::OPTIONS => get_option_field(self.options.as_ref()?, idx),
            tag::message::ONEOF_DECL => {
                let oneof = self.oneof_decl.get(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::Oneof(oneof.clone()))
                } else {
                    None
                }
            }
            // tag::message::RESERVED_RANGE => {
            //     let reserved_range = self.reserved_range.get(idx)?;
            //     if path.is_empty() {
            //         Some(PathedDescriptor::ReservedRange(reserved_range.clone()))
            //     } else {
            //         None
            //     }
            // }
            tag::message::RESERVED_NAME => {
                let reserved_name = self.reserved_name.get(idx)?;
                if path.is_empty() {
                    Some(PathedDescriptor::ReservedName(reserved_name.clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}