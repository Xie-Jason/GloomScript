## GloomScript

**本项目正在开发当中** |
**This project still under development**

### About

本项目是GloomScript语言的解释器实现，解释器前端包含手动实现的 lexer、parser 和 analyzer，其后端曾经是一个AST解释器， 
但现在，我正在添加字节码生成以及字节码解释器以更换AST解释器，使其能成为一个GloomScript语言的程序虚拟机。
在不久的将来，本项目会添加异步运行时、协程 以及一些标准库。

This project is an interpreter implementation of GloomScript in Rust,
which has a manually implemented frontend consist of lexer parser and analyzer, before recent, the backend is an AST interpreter.
but now, I am adding bytecode generation and bytecode interpreter after remove the AST interpreter,
which will make this project come into being a program virtual machine of GloomScript.

### GloomScript language

GloomScript 是一个多编程范式、静态类型、基于表达式的脚本语言。
本项目接近完成时，将介绍GloomScript的详细信息。
你可以在这个目录前瞻GloomScript [/gloom](https://github.com/Xie-Jason/GloomScript/tree/master/gloom)

如果项目处于稳定阶段，那么其中出现的代码应当是解释器可执行的。

GloomScript is a expression-based script language with multi normal form and static type system.
The details of GloomScript will be presented after most work of this project are finished.
You could look forward the GloomScript in this directory [/gloom](https://github.com/Xie-Jason/GloomScript/tree/master/gloom).

All codes in this directory are supported yet by the interpreter if the project is stable.

### Contribution

本项目欢迎各位的贡献，你可以阅读这个文件来获取基础的开发信息 [/doc/dev.md]("https://github.com/Xie-Jason/GloomScript/blob/master/doc/dev.md")

This project welcome contributions, you could read this file to get some basic information of development [/doc/dev.md]("https://github.com/Xie-Jason/GloomScript/blob/master/doc/dev.md")