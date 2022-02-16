use crate::frontend::ops::BinOp;
use crate::frontend::token::Token;
use crate::obj::func::ReturnType;
use crate::obj::types::DataType;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("{info} variable name {symbol} already exist")]
    VarAlreadyOccupied { info: String, symbol: String },

    #[error("function name {symbol} already exist of {typ}")]
    FnAlreadyOccupied { symbol: String, typ: String },

    #[error("type name {typ} already exists")]
    TypeAlreadyOccupied { typ: String, occupy: String },

    #[error("{info} mismatched args num, expect {expect}, found {found} in func {func_name} {func_type}")]
    MismatchedArgsNum {
        info: String,
        func_name: String,
        func_type: DataType,
        expect: usize,
        found: usize,
    },

    #[error("{info} mismatched declared type of variable '{var}', expect {expect}, found {found}")]
    VarDeclMismatchedType {
        info: String,
        var: String,
        expect: DataType,
        found: DataType,
    },

    #[error("{info} expect return {expect}, found {found}")]
    MismatchedReturnType {
        info: String,
        expect: ReturnType,
        found: ReturnType,
    },

    #[error("{info} expect result type of if-else is {expect}, found {found}")]
    MismatchedIfElseResultType {
        info: String,
        expect: ReturnType,
        found: ReturnType,
    },

    #[error("{info} unexpected 'break' in non-loop block line {line}")]
    UnexpectBreak { info: String, line: u16 },

    #[error("{info} unexpected 'continue' in non-loop block line {line}")]
    UnexpectContinue { info: String, line: u16 },

    #[error("{info} line {line}, static variable can't declared in loop or is-else block")]
    StaticInLoopOfIfElse { info: String, line: u16 },

    #[error("unsupported syntax {0}")]
    UnsupportedSyntax(String),

    #[error(
        "{info} line {line}, the condition of {no}st if-else branch have non-bool type {found}"
    )]
    IfConditionNotBool {
        info: String,
        no: usize,
        line: u16,
        found: DataType,
    },

    #[error("{info} line {line}, the condition of while-loop have non-bool type {found}")]
    WhileConditionNotBool {
        info: String,
        line: u16,
        found: DataType,
    },

    #[error("{info} expect a type, found void, in {no}st expression of array literal : {elem}")]
    VoidInArray {
        info: String,
        no: usize,
        elem: String,
    },

    #[error("{info} the object construction expect a Class as type mark, found {wrong_type}")]
    UnexpectedNewClassMark { info: String, wrong_type: DataType },

    #[error("{info} the fields of class have {expect}, found {found} in the construction list")]
    MismatchedNewFieldNum {
        info: String,
        found: usize,
        expect: usize,
    },

    #[error("{info} unknown field name {name} in class {class}, maybe it exists but not public")]
    UnknownField {
        info: String,
        name: String,
        class: String,
    },

    #[error("{info} line {line}, undefined variable {var}")]
    UndefinedVar {
        info: String,
        line: u16,
        var: String,
    },

    #[error("{info} mismatched type when assign, expression type {found} do not belongs to left value type {expect}")]
    AssignMismatchedType {
        info: String,
        found: DataType,
        expect: DataType,
    },

    #[error("{info} the left value of operator {ops} should be a int or num, found {found}")]
    CalcInplaceLeftMismatchedType {
        info: String,
        ops: Token,
        found: DataType,
    },

    #[error("{info} the operated value of operator {ops} should be a int or num, found {found}")]
    CalcInplaceRightMismatchedType {
        info: String,
        ops: Token,
        found: DataType,
    },

    #[error("{info} can't find any field in type : {typ}")]
    NoFieldType { info: String, typ: DataType },

    #[error("{info} field {name} in {typ} is not public")]
    FieldNotPublic {
        info: String,
        name: String,
        typ: String,
        help: &'static str,
    },

    #[error(
        "{info} the field {name} of {class} need a value/object with {expect} type, found {found}"
    )]
    MismatchedFieldType {
        info: String,
        name: String,
        expect: DataType,
        found: DataType,
        class: String,
    },

    #[error("{info} mismatch argument type of {idx}st argument when call function {func}, expect {expect} found {found}")]
    MismatchedArgType {
        info: String,
        idx: usize,
        func: String,
        expect: DataType,
        found: DataType,
    },

    #[error(
        "{info} Type {typ} is not public, you can't use it except in the file where it's defined"
    )]
    UsedPrivateType { info: String, typ: String },

    #[error("{info} function {func} is not public, you can't use it except in the file where it's defined")]
    UsedPrivateFunc {
        info: String,
        func: String,
        typ: String,
    },

    #[error("{info} variable type cast error, you can't cast a var from {from} to {to}")]
    WrongCast {
        info: String,
        from: DataType,
        to: DataType,
    },

    #[error("{info} not found function {func} in type {typ}, or it is not public")]
    FuncNotFound {
        info: String,
        func: String,
        typ: String,
    },

    #[error(
        "{info} function {func} call return void but some chained operation are followed behind"
    )]
    ChainAfterVoid { info: String, func: String },

    #[error("{info} you may forgot import builtin type {typ} in std library")]
    UnImportedBuiltinType { info: String, typ: String },

    #[error("could not access the member function of Interface {inter} by static-func-call")]
    AccessInterfaceEmptyFn { info: String, inter: String },

    #[error("{info} the {typ} type is not a func type, you can't use a '()' to call it")]
    CannotCallNonFnType { info: String, typ: DataType },

    #[error("type '{typ}' not found")]
    UnknownType { typ: String },

    #[error("{info} function {func} of class {class} is not a non-static function, which don't have 'self' as first parameter")]
    StaticFnNotMethod {
        info: String,
        func: String,
        class: String,
    },

    #[error("{info} basic data type value can't be caller of member function, found {typ} value as caller call function '{func}'")]
    BasicTypeAsCaller {
        info: String,
        typ: DataType,
        func: String,
    },

    #[error("{info} mismatched argument type in first argument 'self' of function {func} call, expect {expect}, found {found}")]
    MismatchedSelfType {
        info: String,
        func: String,
        expect: DataType,
        found: DataType,
    },

    #[error("{info} binary operator {op} have wrong operand type {typ}")]
    BinOpWrongType {
        info: String,
        op: BinOp,
        typ: DataType,
    },

    #[error("{info} binary operator '==' have wrong operand type, {typ1} and {typ2}")]
    EqualsWrongType {
        info: String,
        typ1: DataType,
        typ2: DataType,
    },

    #[error("{info} expect int in start,end and step of for-in range loop, found {typ} in {no}st range arg")]
    RangeWrongArgType {
        info: String,
        no: usize,
        typ: DataType,
    },

    #[error("{info} can't apply for-in iteration in type {typ}")]
    CannotIter { info: String, typ: DataType },

    #[error("{info} declared parent class {parent} of {class} is not a class")]
    ParentNotAClass {
        info: String,
        class: String,
        parent: String,
    },

    #[error("interface {inter} that implemented by class {class} is in fact not an interface but a {typ}")]
    ImplNotInterface {
        inter: String,
        class: String,
        typ: String,
    },

    #[error("wrong {no}st parameter 'self' occurs in function {func} of {typ}")]
    UnexpectedSelf {
        no: usize,
        func: String,
        typ: String,
    },

    #[error("declared parent interface {parent} of {inter} is not interface")]
    InterfaceExtendNonInterface { parent: String, inter: String },

    #[error("{info} generic type error {error}")]
    GenericError{
        info : String,
        error : String,
    },

    #[error("function {func} that declared at interface {interface} need be implemented by class {class}")]
    FnNotImpl{
        class : String,
        interface : String,
        func : String,
    },

    #[error("the return type of function {func} that declared in interface {inter} is {expect} but in fact found {found} in the implemented class {class}")]
    MismatchedImplReturnType{
        func : String,
        inter : String,
        class : String,
        expect : ReturnType,
        found : ReturnType
    },

    #[error("the {idx}st param type of function {func} that declared in interface {inter} is {expect} but in fact found {found} in the implemented class {class}")]
    MismatchedImplParamType{
        idx : usize,
        func : String,
        inter : String,
        class : String,
        expect : DataType,
        found : DataType
    },

    #[error("the params length of function {func} that declared in interface {inter} is different from the implementation function in class {class}, expect {expect}, found {found}")]
    MismatchImplParamLen{
        func : String,
        inter : String,
        class : String,
        expect : usize,
        found : usize
    }
}
