use std::{ffi::{c_char, c_void}, hash::Hash, ops::Not, sync::Arc};

#[repr(C)]
pub enum FFIStatus {
    Ok, Err,
}

#[repr(C)]
pub struct FFIResult {
    pub status: FFIStatus,
    pub data_ptr: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FFISlice {
    pub ptr: *const c_char,
    pub len: usize,
}

#[repr(C)]
pub enum FFITypeTag {
    Number,
    Boolean,
    String,
    Vector,
    Object,
}

#[repr(C)]
pub union FFiDataUnion {
    pub number: i64,
    pub boolean: bool,
    pub string: FFISlice,
    pub vector: FFISlice,
    pub object_ptr: *mut std::ffi::c_void,
}

#[repr(C)]
pub struct FFIValue {
    pub tag: FFITypeTag,
    pub data: FFiDataUnion,
}

#[derive(Clone)]
pub enum RDLTypes {
    String(String),
    Number(i64),
    Vector(Vec<RDLTypes>),
    Boolean(bool),
    Object(Arc<tokio::sync::Mutex<dyn Object + Send + Sync>>),
}

impl RDLTypes {
    pub fn as_i64(&self) -> Result<i64, String> {
        match self {
            Self::Number(n) => Ok(*n),
            _ => Err("Can't convert this type to i64".to_string()),
        }
    }

    pub fn as_u64(&self) -> Result<u64, String> {
        self.as_i64().map(|i| i as u64)
    }

    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Self::Boolean(b) => Ok(*b),
            Self::Number(0) => Ok(false),
            Self::Number(1) => Ok(true),
            Self::Number(n) => Err(format!("Type error: Number {} cannot be boolean", n)),
            _ => Err("Can't convert this type to bool".to_string()),
        }
    }

    pub fn as_vec(&self) -> Result<Vec<RDLTypes>, String> {
        match self {
            Self::Vector(v) => Ok(v.to_vec()),
            _ => Err("Can't convert this type to bool".to_string()),
        }
    }

    pub fn to_ffi(&self) -> FFIValue {
        match self {
            Self::Number(n) => FFIValue {
                tag: FFITypeTag::Number,
                data: FFiDataUnion { number: *n },
            },
            Self::Boolean(b) => FFIValue {
                tag: FFITypeTag::Boolean,
                data: FFiDataUnion { boolean: *b },
            },
            Self::String(s) => FFIValue {
                tag: FFITypeTag::String,
                data: FFiDataUnion {
                    string: FFISlice { ptr: s.as_ptr() as *const c_char, len: s.len() },
                },
            },
            Self::Vector(v) => FFIValue {
                tag: FFITypeTag::Vector,
                data: FFiDataUnion {
                    vector: FFISlice { ptr: v.as_ptr() as *const c_char, len: v.len() },
                },
            },
            Self::Object(o) => FFIValue {
                tag: FFITypeTag::Object,
                data: FFiDataUnion {
                    object_ptr: Arc::into_raw(o.clone()) as *mut c_void
                }
            }
        }
    }
}

impl Not for RDLTypes {
    type Output = bool;

    fn not(self) -> Self::Output {
        match self {
            RDLTypes::Boolean(b) => b,
            RDLTypes::Number(n) => n == 1,
            _ => false,
        }
    }
}

impl PartialOrd for RDLTypes {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (RDLTypes::Number(n1), RDLTypes::Number(n2)) => {
                if *n1 == *n2 {
                    return Some(std::cmp::Ordering::Equal);
                }

                if *n1 > *n2 {
                    return Some(std::cmp::Ordering::Greater);
                }

                if *n1 < *n2 {
                    return Some(std::cmp::Ordering::Less);
                }

                None
            }
            (RDLTypes::String(s1), RDLTypes::String(s2)) => {
                if *s1 == *s2 {
                    return Some(std::cmp::Ordering::Equal);
                }

                if (*s1).len() > (*s2).len() {
                    return Some(std::cmp::Ordering::Greater);
                }

                if (*s1).len() < (*s2).len() {
                    return Some(std::cmp::Ordering::Less);
                }

                None
            }
            (RDLTypes::Boolean(b1), RDLTypes::Boolean(b2)) => {
                if *b1 && *b2 {
                    return Some(std::cmp::Ordering::Equal);
                }

                if !*b1 && *b2 {
                    return Some(std::cmp::Ordering::Less);
                }

                if *b1 && !*b2 {
                    return Some(std::cmp::Ordering::Greater);
                }

                None
            }
            _ => None
        }
    }
}

impl Hash for RDLTypes {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);

        match self {
            Self::String(s) => s.hash(state),
            Self::Number(n) => n.hash(state),
            Self::Boolean(b) => b.hash(state),
            Self::Vector(v) => v.hash(state),
            Self::Object(obj) => {
                let ptr = Arc::as_ptr(obj) as *const () as usize;
                ptr.hash(state);
            }
        }
    }
}

impl Eq for RDLTypes {}

impl PartialEq for RDLTypes {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean(b1), Self::Boolean(b2)) => *b1 == *b2,
            (Self::Number(n1), Self::Number(n2)) => *n1 == *n2,
            (Self::String(s1), Self::String(s2)) => *s1 == *s2,
            (Self::Vector(v1), Self::Vector(v2)) => *v1 == *v2,
            (Self::Object(obj1), Self::Object(obj2)) => {
                Arc::ptr_eq(obj1, obj2)
            },
            _ => false,
        }
    }
}

impl From<RDLTypes> for u16 {
    fn from(value: RDLTypes) -> Self {
        match value {
            RDLTypes::Boolean(b) => if b { 1 } else { 0 },
            RDLTypes::Number(n) => n as u16,
            RDLTypes::String(s) => s.parse::<u16>().unwrap_or(0),
            RDLTypes::Vector(v) => v.into_iter().map(u16::from).sum(),
            RDLTypes::Object(_) => 0,
        }
    }
}

impl From<usize> for RDLTypes {
    fn from(value: usize) -> Self {
        RDLTypes::Number(value as i64)
    }
}

impl From<u16> for RDLTypes {
    fn from(value: u16) -> Self {
        RDLTypes::Number(value as i64)
    }
}

impl From<&RDLTypes> for u16 {
    fn from(value: &RDLTypes) -> Self {
        match value {
            RDLTypes::Boolean(b) => if *b { 1 } else { 0 },
            RDLTypes::Number(n) => *n as u16,
            RDLTypes::String(s) => s.parse::<u16>().unwrap_or(0),
            RDLTypes::Vector(v) => v.iter().map(u16::from).sum(), 
            RDLTypes::Object(_) => 0,
        }
    }
}

impl From<String> for RDLTypes {
    fn from(value: String) -> Self {
        RDLTypes::String(value)
    }
}

impl From<&str> for RDLTypes {
    fn from(value: &str) -> Self {
        RDLTypes::String(value.to_string())
    }
}

impl From<i64> for RDLTypes {
    fn from(value: i64) -> Self {
        RDLTypes::Number(value)
    }
}

impl From<bool> for RDLTypes {
    fn from(value: bool) -> Self {
        RDLTypes::Boolean(value)
    }
}

impl std::fmt::Debug for RDLTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RDLTypes::String(s) => f.debug_tuple("String").field(s).finish(),
            RDLTypes::Number(n) => f.debug_tuple("Number").field(n).finish(),
            RDLTypes::Boolean(b) => f.debug_tuple("Boolean").field(b).finish(),
            RDLTypes::Vector(v) => f.debug_tuple("Vector").field(v).finish(),
            RDLTypes::Object(_) => f.debug_tuple("Object").field(&"dyn Object").finish(),
        }
    }
}

impl std::fmt::Display for RDLTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RDLTypes::String(s) => f.write_str(s),
            RDLTypes::Number(n) => write!(f, "{n}"),
            RDLTypes::Boolean(b) => write!(f, "{b}"),
            RDLTypes::Object(_) => f.write_str("[object Object]"),
            RDLTypes::Vector(v) => {
                f.write_str("[")?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 { f.write_str(", ")?; }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
        }
    }
}

pub trait Object: 'static + Send + Sync{
    fn name(&self) -> &'static str;
    fn methods(&self) -> Vec<&str>;
    fn properties(&self) -> Vec<&str>;
    fn method_exist(&self, name: &str) -> bool;
    fn call_method(&mut self, name: &str, args: Vec<RDLTypes>) -> Result<RDLTypes, String>;
    fn property_exist(&self, name: &str) -> bool;
    fn get_property(&self, name: &str) -> RDLTypes;
}