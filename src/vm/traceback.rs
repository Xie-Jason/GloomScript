use std::process::abort;

// 在raise的unsafe里面使用了
static mut IN_EXCEPTION:bool = false;
// raise_exception()可以兼任的exception
pub trait Exception{
    /// 自行处理所有问题in raise()
    fn raise(&self) -> !;
}
// 异常系统关键struct
#[derive(Debug)]
pub struct GloomException{
    file:String,
    module:String,
    line:usize,
    message:&'static str,
}

impl GloomException{
    pub fn new_empty_exception() ->Self{
        Self{
            file:"".to_string(),
            module:"".to_string(),
            line:0,
            message:""
        }
    }
    pub fn new(file:String,module:String,line:usize,message:&'static str) -> Self{
        Self{
            file:file,
            module:module,
            line:line,
            message:message
        }
    }
    fn raise(&self) -> !{
        // unsafe if prepare for read the static IN_EXCEPTION,it is always safe
        // if we meet two exception in a time,abort the programma
        unsafe{
            if !IN_EXCEPTION{
                IN_EXCEPTION = true;
                eprintln!("TraceBack:\nIn file{},module{},line {}\nException:{}",
                self.file,self.module,self.line,self.message)
                // TODO : 执行清理
            } // have already in exception
            abort();
        }
    }
}
impl Exception for GloomException{
    /// '''
    /// let exception = GloomException::new("example.gs".to_string(),)
    fn raise(&self) -> !{
        // unsafe if prepare for read the static IN_EXCEPTION,it is always safe
        // if we meet two exception in a time,abort the programma
        unsafe{
            if !IN_EXCEPTION{
                IN_EXCEPTION = true;
                eprintln!("TraceBack:\nIn file{},module{},line {}\nException:{}",
                self.file,self.module,self.line,self.message)
                // TODO : 执行清理
            } // have already in exception
            abort();
        }
    }
}
pub(crate) fn unwrap<T>(t:Option<T>) -> T{
    match t{
        Some(v) => v,
        // A exception is raised.
        _ => GloomException::new_empty_exception().raise()
    }
}

pub(crate) fn raise_exception<T:Exception>(exception:T) -> !{
    exception.raise();
}