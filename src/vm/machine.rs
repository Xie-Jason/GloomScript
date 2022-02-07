use crate::exec::static_table::StaticTable;
use crate::frontend::status::GloomStatus;
use crate::vm::constant::ConstantPool;

pub struct GloomVM{
    static_table  : StaticTable,
    constant_pool : ConstantPool,
    status : GloomStatus,
}

impl GloomVM {
    pub fn new(static_table: StaticTable, constant_pool: ConstantPool, status: GloomStatus) -> Self {
        GloomVM { static_table, constant_pool, status }
    }
    pub fn run(mut self){

    }
}