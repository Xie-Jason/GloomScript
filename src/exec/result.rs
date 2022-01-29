use crate::exec::value::Value;

#[derive(Debug)]
pub enum GloomResult {
    // the return of function call
    Return(Value),
    ReturnVoid,
    // the result of expression
    Value(Value),
    ValueVoid,
    // the result of while loop or for loop
    Break(Value),
    BreakVoid,
    Continue,
    // the result value of if-else
    IfElseResult(Value),
    IfElseVoid
}

impl GloomResult {
    #[inline]
    pub fn assert_into_value(self) -> Value{
        match self {
            GloomResult::Value(val) => val,
            _ => panic!("{:?}",self),
        }
    }
}
