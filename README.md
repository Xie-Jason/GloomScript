## GloomScript

**本项目正在开发当中** |
**This project still under development**

### About

本项目是GloomScript语言的解释器实现，解释器前端包含手动实现的 lexer、parser 和 analyzer 以及 bytecode-compiler，其后端是一个字节码解释器。 最近，本项目会添加异步运行时、协程
以及一些标准库。

This project is an interpreter implementation of GloomScript in Rust, which has a manually implemented frontend consist
of lexer parser analyzer and bytecode-compiler with a bytecode interpreter in backend. recently, this project will add
async-runtime coroutine and some standard lib

### GloomScript language

GloomScript 是一个多编程范式、静态类型、基于表达式的脚本语言，同样由我设计。

这个目录有一些用于测试和示例的GloomScript代码： [/gloom](https://github.com/Xie-Jason/GloomScript/tree/master/gloom)
如果项目处于稳定阶段，那么其中出现的代码应当是解释器可执行的。

GloomScript is a expression-based script language with multi normal form and static type system.

This directory has some files that used to test and
example： [/gloom](https://github.com/Xie-Jason/GloomScript/tree/master/gloom). All codes in this directory are supported
yet by the interpreter if the project is stable.

### Contribution

本项目欢迎各位的贡献，你可以阅读这个文件来获取基础的开发信息 [/doc/dev.md](https://github.com/Xie-Jason/GloomScript/blob/master/doc/dev.md)

This project welcome contributions, you could read this file to get some basic information of
development [/doc/dev.md](https://github.com/Xie-Jason/GloomScript/blob/master/doc/dev.md)