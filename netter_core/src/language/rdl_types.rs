// use std::ffi::{c_char, c_void};
// use std::hash::Hash;
// use std::str::FromStr;
// use std::sync::Arc;
// use crate::language::interpreter::Object;
// use crate::language::interpreter::builtin::plugin::{FFISlice, FFITypeTag, FFIValue, FFiDataUnion};
// use crate::parser_error;
// use crate::language::Error;

// #[derive(Clone)]
// pub enum RDLTypes {
//     String(String),
//     Number(i64),
//     Vector(Vec<RDLTypes>),
//     Boolean(bool),
//     Object(Arc<tokio::sync::Mutex<dyn Object + Send + Sync>>),
// }

// impl RDLTypes {
//     pub fn to_ffi(&self) -> FFIValue {
//         match self {
//             Self::Number(n) => FFIValue {
//                 tag: FFITypeTag::Number,
//                 data: FFiDataUnion { number: *n },
//             },
//             Self::Boolean(b) => FFIValue {
//                 tag: FFITypeTag::Boolean,
//                 data: FFiDataUnion { boolean: *b },
//             },
//             Self::String(s) => FFIValue {
//                 tag: FFITypeTag::String,
//                 data: FFiDataUnion {
//                     string: FFISlice { ptr: s.as_ptr() as *const c_char, len: s.len() },
//                 },
//             },
//             Self::Vector(v) => FFIValue {
//                 tag: FFITypeTag::Vector,
//                 data: FFiDataUnion {
//                     vector: FFISlice { ptr: v.as_ptr() as *const c_char, len: v.len() },
//                 },
//             },
//             Self::Object(o) => FFIValue {
//                 tag: FFITypeTag::Object,
//                 data: FFiDataUnion {
//                     object_ptr: Arc::into_raw(o.clone()) as *mut c_void
//                 }
//             }
//         }
//     }
// }

// impl Hash for RDLTypes {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         std::mem::discriminant(self).hash(state);

//         match self {
//             Self::String(s) => s.hash(state),
//             Self::Number(n) => n.hash(state),
//             Self::Boolean(b) => b.hash(state),
//             Self::Vector(v) => v.hash(state),
//             Self::Object(obj) => {
//                 let ptr = Arc::as_ptr(obj) as *const () as usize;
//                 ptr.hash(state);
//             }
//         }
//     }
// }

// impl Eq for RDLTypes {}

// impl PartialEq for RDLTypes {
//     fn eq(&self, other: &Self) -> bool {
//         match (self, other) {
//             (Self::Boolean(b1), Self::Boolean(b2)) => *b1 == *b2,
//             (Self::Number(n1), Self::Number(n2)) => *n1 == *n2,
//             (Self::String(s1), Self::String(s2)) => *s1 == *s2,
//             (Self::Vector(v1), Self::Vector(v2)) => *v1 == *v2,
//             (Self::Object(obj1), Self::Object(obj2)) => {
//                 Arc::ptr_eq(obj1, obj2)
//             },
//             _ => false,
//         }
//     }
// }

// impl From<RDLTypes> for u16 {
//     fn from(value: RDLTypes) -> Self {
//         match value {
//             RDLTypes::Boolean(b) => if b { 1 } else { 0 },
//             RDLTypes::Number(n) => n as u16,
//             RDLTypes::String(s) => s.parse::<u16>().unwrap_or(0),
//             RDLTypes::Vector(v) => v.into_iter().map(u16::from).sum(),
//             RDLTypes::Object(_) => 0,
//         }
//     }
// }

// impl TryFrom<RDLTypes> for String {
//     type Error = Error;

//     fn try_from(value: RDLTypes) -> Result<Self, Error> {
//         if let RDLTypes::String(s) = value {
//             Ok(s)
//         } else {
//             parser_error!(format!("Error with conversion! IN TryFrom<RDLTypes> for String\n{value}\n"), line!() as usize, column!() as usize)
//         }
//     }
// }

// impl TryFrom<RDLTypes> for i64 {
//     type Error = Error;

//     fn try_from(value: RDLTypes) -> Result<Self, Error> {
//         if let RDLTypes::Number(n) = value {
//             Ok(n)
//         } else {
//             parser_error!("Error with conversion! IN TryFrom<RDLTypes> for i64", line!() as usize, column!() as usize)
//         }
//     }
// }

// impl TryFrom<RDLTypes> for Vec<RDLTypes> {
//     type Error = Error;

//     fn try_from(value: RDLTypes) -> Result<Self, Error> {
//         if let RDLTypes::Vector(v) = value {
//             Ok(v)
//         } else {
//             parser_error!("Error with conversion! IN TryFrom<RDLTypes> for Vec<RDLTypes>", line!() as usize, column!() as usize)
//         }
//     }
// }

// impl TryFrom<RDLTypes> for bool {
//     type Error = Error;

//     fn try_from(value: RDLTypes) -> Result<Self, Error> {
//         if let RDLTypes::Boolean(b) = value {
//             Ok(b)
//         } else {
//             parser_error!("Error with conversion! IN TryFrom<RDLTypes> for bool", line!() as usize, column!() as usize)
//         }
//     }
// }

// // impl TryFrom<RDLTypes> for f64 {
// //     type Error = Error;
// //
// //     fn try_from(value: RDLTypes) -> Result<Self, Error> {
// //         if let RDLTypes::Number(n) = value {
// //             Ok(n as f64)
// //         } else {
// //             parser_error!("Error with conversion!", line!() as usize, column!() as usize)
// //         }
// //     }
// // }

// impl TryFrom<RDLTypes> for i128 {
//     type Error = Error;

//     fn try_from(value: RDLTypes) -> Result<Self, Error> {
//         if let RDLTypes::Number(n) = value {
//             Ok(n as i128)
//         } else {
//             parser_error!("Error with conversion! IN TryFrom<RDLTypes> for i128", line!() as usize, column!() as usize)
//         }
//     }
// }

// impl TryFrom<RDLTypes> for usize {
//     type Error = Error;

//     fn try_from(value: RDLTypes) -> Result<Self, Error> {
//         if let RDLTypes::Number(n) = value {
//             Ok(n as usize)
//         } else {
//             parser_error!("Error with conversion! IN TryFrom<RDLTypes> for usize", line!() as usize, column!() as usize)
//         }
//     }
// }

// impl From<usize> for RDLTypes {
//     fn from(value: usize) -> Self {
//         RDLTypes::Number(value as i64)
//     }
// }

// impl From<u16> for RDLTypes {
//     fn from(value: u16) -> Self {
//         RDLTypes::Number(value as i64)
//     }
// }

// impl From<&RDLTypes> for u16 {
//     fn from(value: &RDLTypes) -> Self {
//         match value {
//             RDLTypes::Boolean(b) => if *b { 1 } else { 0 },
//             RDLTypes::Number(n) => *n as u16,
//             RDLTypes::String(s) => s.parse::<u16>().unwrap_or(0),
//             RDLTypes::Vector(v) => v.iter().map(u16::from).sum(), 
//             RDLTypes::Object(_) => 0,
//         }
//     }
// }

// impl From<String> for RDLTypes {
//     fn from(value: String) -> Self {
//         RDLTypes::String(value)
//     }
// }

// impl From<&str> for RDLTypes {
//     fn from(value: &str) -> Self {
//         RDLTypes::String(value.to_string())
//     }
// }

// impl From<i64> for RDLTypes {
//     fn from(value: i64) -> Self {
//         RDLTypes::Number(value)
//     }
// }

// impl From<bool> for RDLTypes {
//     fn from(value: bool) -> Self {
//         RDLTypes::Boolean(value)
//     }
// }

// impl FromStr for RDLTypes {
//     type Err = Error;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         Ok(RDLTypes::String(s.to_string()))
//     }
// }

// impl std::fmt::Debug for RDLTypes {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             RDLTypes::String(s) => f.debug_tuple("String").field(s).finish(),
//             RDLTypes::Number(n) => f.debug_tuple("Number").field(n).finish(),
//             RDLTypes::Boolean(b) => f.debug_tuple("Boolean").field(b).finish(),
//             RDLTypes::Vector(v) => f.debug_tuple("Vector").field(v).finish(),
//             RDLTypes::Object(_) => f.debug_tuple("Object").field(&"dyn Object").finish(),
//         }
//     }
// }

// impl std::fmt::Display for RDLTypes {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         match self {
//             RDLTypes::String(s) => f.write_str(s),
//             RDLTypes::Number(n) => write!(f, "{n}"),
//             RDLTypes::Boolean(b) => write!(f, "{b}"),
//             RDLTypes::Object(_) => f.write_str("[object Object]"),
//             RDLTypes::Vector(v) => {
//                 f.write_str("[")?;
//                 for (i, item) in v.iter().enumerate() {
//                     if i > 0 { f.write_str(", ")?; }
//                     write!(f, "{item}")?;
//                 }
//                 f.write_str("]")
//             }
//         }
//     }
// }