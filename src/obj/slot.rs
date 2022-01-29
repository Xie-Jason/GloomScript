use std::mem::{ManuallyDrop};
use crate::obj::object::GloomObjRef;

#[repr(C)]
pub union Slot {
    pub(crate) int : [i64;2],
    pub(crate) num : [f64;2],
    pub(crate) ch  : [char;4],
    pub(crate) bl  : [bool;16],
    pub(crate) rf  : ManuallyDrop<GloomObjRef>,
}