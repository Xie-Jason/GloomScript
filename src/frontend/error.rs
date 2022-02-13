use thiserror::Error;
use crate::frontend::token::Token;
use crate::obj::func::ReturnType;
use crate::obj::types::{DataType};

#[derive(Error,Debug)]
pub enum AnalysisError{
    #[error("{info} variable name {symbol} already exist")]
    SymbolAlreadyOccupied{
        info : String,
        symbol : String
    },

    #[error("{info} mismatched args num, expect {expect}, found {found} in func {func_name} {func_type}")]
    MismatchedArgsNum{
        info : String,
        func_name : String,
        func_type : DataType,
        expect : usize,
        found : usize,
    },

    #[error("{info} mismatched declared type of variable '{var}', expect {expect}, found {found}")]
    VarDeclMismatchedType{
        info : String,
        var : String,
        expect : DataType,
        found : DataType,
    },

    #[error("{info} expect return {expect}, found {found}")]
    MismatchedReturnType{
        info : String,
        expect : ReturnType,
        found : ReturnType
    },

    #[error("{info} expect result type of if-else is {expect}, found {found}")]
    MismatchedIfElseResultType{
        info : String,
        expect : ReturnType,
        found : ReturnType
    },

    #[error("{info} unexpected 'break' in non-loop block line {line}")]
    UnexpectBreak {
        info : String,
        line : u16
    },

    #[error("{info} unexpected 'continue' in non-loop block line {line}")]
    UnexpectContinue{
        info : String,
        line : u16
    },

    #[error("{info} line {line}, static variable can't declared in loop or is-else block")]
    StaticInLoopOfIfElse{
        info : String,
        line : u16
    },
    #[error("unsupported syntax {0}",)]
    UnsupportedSyntax(String),

    #[error("{info} line {line}, the condition of {no}st if-else branch have non-bool type {found}")]
    ConditionNotBool{
        info : String,
        no : usize,
        line : u16,
        found : DataType
    },

    #[error("{info} expect a type, found void, in {no}st expression of array literal : {elem}")]
    VoidInArray{
        info : String,
        no : usize,
        elem : String
    },

    #[error("{info} the object construction expect a Class as type mark, found {wrong_type}")]
    UnexpectedNewClassMark {
        info : String,
        wrong_type : DataType
    },

    #[error("{info} the fields of class have {expect}, found {found} in the construction list")]
    MismatchedNewFieldNum{
        info : String,
        found : usize,
        expect : usize
    },

    #[error("{info} unknown field name {name} in class {class}, maybe it exists but not public")]
    UnknownField {
        info : String,
        name : String,
        class : String,
    },

    #[error("{info} line {line}, undefined variable {var}")]
    UndefinedVar{
        info : String,
        line : u16,
        var : String
    },

    #[error("{info} mismatched type when assign, expression type {found} do not belongs to left value type {expect}")]
    AssignMismatchedType{
        info : String,
        found : DataType,
        expect : DataType
    },

    #[error("{info} the left value of operator {ops} should be a int or num, found {found}")]
    CalcInplaceLeftMismatchedType{
        info : String,
        ops : Token,
        found : DataType
    },

    #[error("{info} the operated value of operator {ops} should be a int or num, found {found}")]
    CalcInplaceRightMismatchedType{
        info : String,
        ops : Token,
        found : DataType
    },

    #[error("{info} can't find any field in type : {typ}")]
    NoFieldType{
        info : String,
        typ : DataType
    },

    #[error("{info} field {name} in {typ} is not public")]
    FieldNotPublic{
        info : String,
        name : String,
        typ : String,
        help : &'static str
    },

    #[error("{info} the field {name} of {class} need a value/object with {expect} type, found {found}")]
    MismatchedFieldType{
        info : String,
        name : String,
        expect : DataType,
        found : DataType,
        class : String
    },

    #[error("{info} mismatch argument type of {idx}st argument when call function {func}, expect {expect} found {found}")]
    MismatchedArgType{
        info : String,
        idx : usize,
        func : String,
        expect : DataType,
        found : DataType
    },
}