use std::fmt::{Debug, Formatter};
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

use crate::obj::object::GloomObjRef;

pub enum Slot {
    Null,
    Int([i64; 2]),
    Num([f64; 2]),
    Char([char; 4]),
    Bool([bool; 16]),
    Ref(ManuallyDrop<GloomObjRef>),
}

impl Debug for Slot {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Slot::Null => write!(f, "null"),
            Slot::Int(arr) => write!(f, "{:?}", arr),
            Slot::Num(arr) => write!(f, "{:?}", arr),
            Slot::Char(arr) => write!(f, "{:?}", arr),
            Slot::Bool(arr) => write!(f, "{:?}", arr),
            Slot::Ref(rf) => write!(f, "{:?}", ManuallyDrop::deref(rf)),
        }
    }
}

impl Slot {
    #[inline(always)]
    pub fn get_ref(&self) -> &GloomObjRef {
        if let Slot::Ref(rf) = self {
            rf.deref()
        } else {
            panic!()
        }
    }

    #[inline(always)]
    pub fn into_ref(self) -> ManuallyDrop<GloomObjRef> {
        if let Slot::Ref(rf) = self {
            rf
        } else {
            panic!()
        }
    }
    #[inline(always)]
    pub fn take(&mut self) -> Slot {
        std::mem::replace(self, Slot::Null)
    }
    #[inline(always)]
    pub fn replace(&mut self, slot: Slot) -> Slot {
        std::mem::replace(self, slot)
    }
    #[inline(always)]
    pub fn set_int(&mut self, sub_idx: u8, val: i64) {
        match self {
            Slot::Int(arr) => arr[sub_idx as usize] = val,
            Slot::Null => {
                let mut arr = [0; 2];
                arr[sub_idx as usize] = val;
                *self = Slot::Int(arr)
            }
            _ => panic!(),
        }
    }
    #[inline(always)]
    pub fn set_num(&mut self, sub_idx: u8, val: f64) {
        match self {
            Slot::Num(arr) => arr[sub_idx as usize] = val,
            Slot::Null => {
                let mut arr = [0.0; 2];
                arr[sub_idx as usize] = val;
                *self = Slot::Num(arr)
            }
            _ => panic!(),
        }
    }
    #[inline(always)]
    pub fn set_char(&mut self, sub_idx: u8, val: char) {
        match self {
            Slot::Char(arr) => arr[sub_idx as usize] = val,
            Slot::Null => {
                let mut arr = [0 as char; 4];
                arr[sub_idx as usize] = val;
                *self = Slot::Char(arr)
            }
            _ => panic!(),
        }
    }
    #[inline(always)]
    pub fn set_bool(&mut self, sub_idx: u8, val: bool) {
        match self {
            Slot::Bool(arr) => arr[sub_idx as usize] = val,
            Slot::Null => {
                let mut arr = [false; 16];
                arr[sub_idx as usize] = val;
                *self = Slot::Bool(arr)
            }
            _ => panic!(),
        }
    }
}
