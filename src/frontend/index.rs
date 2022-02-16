use core::fmt::{Debug, Formatter};

use crate::obj::types::DataType;

#[derive(Clone)]
pub struct SlotIndexer {
    max_idx: i16,
    int_curr_slot_idx: i16,
    int_curr_sub_idx: i16,
    num_curr_slot_idx: i16,
    num_curr_sub_idx: i16,
    char_curr_slot_idx: i16,
    char_curr_sub_idx: i16,
    bool_curr_slot_idx: i16,
    bool_curr_sub_idx: i16,
    types: Vec<DataType>,
    drop_vec_stack: Vec<Vec<u16>>,
}

impl SlotIndexer {
    const INT_MAX_SUB_IDX: i16 = 1;
    const NUM_MAX_SUB_IDX: i16 = 1;
    const CHAR_MAX_SUB_IDX: i16 = 3;
    const BOOL_MAX_SUB_IDX: i16 = 15;

    #[inline]
    fn put_int(&mut self) -> (u16, u8) {
        if self.int_curr_slot_idx < 0 || self.int_curr_sub_idx >= SlotIndexer::INT_MAX_SUB_IDX {
            // have no int slot yet or current int slot is full, need alloc a new int slot
            self.max_idx += 1;
            self.types.push(DataType::Int);
            self.int_curr_slot_idx = self.max_idx;
            self.int_curr_sub_idx = 0;
        } else {
            // current int slot is not full
            self.int_curr_sub_idx += 1;
        }
        (self.int_curr_slot_idx as u16, self.int_curr_sub_idx as u8)
    }

    #[inline]
    fn put_num(&mut self) -> (u16, u8) {
        if self.num_curr_slot_idx < 0 || self.num_curr_sub_idx >= SlotIndexer::NUM_MAX_SUB_IDX {
            self.max_idx += 1;
            self.types.push(DataType::Num);
            self.num_curr_slot_idx = self.max_idx;
            self.num_curr_sub_idx = 0;
        } else {
            self.num_curr_sub_idx += 1;
        }
        (self.num_curr_slot_idx as u16, self.num_curr_sub_idx as u8)
    }

    #[inline]
    fn put_char(&mut self) -> (u16, u8) {
        if self.char_curr_slot_idx < 0 || self.char_curr_sub_idx >= SlotIndexer::CHAR_MAX_SUB_IDX {
            self.max_idx += 1;
            self.types.push(DataType::Char);
            self.char_curr_slot_idx = self.max_idx;
            self.char_curr_sub_idx = 0;
        } else {
            self.char_curr_sub_idx += 1;
        }
        (self.char_curr_slot_idx as u16, self.char_curr_sub_idx as u8)
    }

    #[inline]
    fn put_bool(&mut self) -> (u16, u8) {
        if self.bool_curr_slot_idx < 0 || self.bool_curr_sub_idx >= SlotIndexer::BOOL_MAX_SUB_IDX {
            self.max_idx += 1;
            self.types.push(DataType::Bool);
            self.bool_curr_slot_idx = self.max_idx;
            self.bool_curr_sub_idx = 0;
        } else {
            self.bool_curr_sub_idx += 1;
        }
        (self.bool_curr_slot_idx as u16, self.bool_curr_sub_idx as u8)
    }

    #[inline]
    pub fn put(&mut self, data_type: DataType) -> (u16, u8) {
        match data_type {
            DataType::Int => self.put_int(),
            DataType::Num => self.put_num(),
            DataType::Char => self.put_char(),
            DataType::Bool => self.put_bool(),
            ref_type => {
                self.max_idx += 1;
                self.types.push(ref_type);
                self.drop_vec_stack
                    .last_mut()
                    .unwrap()
                    .push(self.max_idx as u16);
                (self.max_idx as u16, 0)
            }
        }
    }

    pub fn enter_sub_block(&mut self) {
        self.drop_vec_stack.push(Vec::new());
    }
    pub fn level_sub_block(&mut self) -> Vec<u16> {
        self.drop_vec_stack.pop().unwrap()
    }

    #[inline]
    pub fn get_type(&self, index: u16) -> &DataType {
        self.types.get(index as usize).unwrap()
    }

    #[inline]
    pub fn size(&self) -> u16 {
        (self.max_idx + 1) as u16
    }

    #[inline]
    pub fn curr_drop_vec(&self) -> &Vec<u16> {
        self.drop_vec_stack.last().unwrap()
    }

    pub fn basic_drop_vec(&mut self) -> Vec<u16> {
        let vec = self.drop_vec_stack.pop().unwrap();
        if self.drop_vec_stack.len() != 0 {
            panic!()
        }
        vec
    }

    pub fn new() -> SlotIndexer {
        SlotIndexer {
            max_idx: -1,
            int_curr_slot_idx: -1,
            int_curr_sub_idx: -1,
            num_curr_slot_idx: -1,
            num_curr_sub_idx: -1,
            char_curr_slot_idx: -1,
            char_curr_sub_idx: -1,
            bool_curr_slot_idx: -1,
            bool_curr_sub_idx: -1,
            types: Vec::new(),
            drop_vec_stack: vec![Vec::new()],
        }
    }
}

impl Debug for SlotIndexer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}", self.types)
    }
}
