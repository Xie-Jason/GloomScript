## GloomScript

### 特点 Trait

- 静态类型语言，强类型检查。
- 支持`OOP`，有类和接口，单继承、多实现。支持元组。对部分内置类型提供泛型支持。
- 多编程范式，支持部分`FP`风格。函数作为一等公民，支持高阶函数、匿名函数、闭包捕获、立即执行函数。*expression-based*， `if-else`是表达式*expression*而非语句*statement*。
- 暂未确定：枚举类支持关联类型以实现`Tagged Union`。模式匹配`pattern match `。

### Data Type

- 基本数据类型|Basic Data Type : `int`  `num` `bool` `char` 。`int`是64位整形；`num`是64位浮点；`char`为32位，这意味着它能容易的支持`Unicode`。

- 用户可以定义的类型有：类|`class`，枚举类|`enum`，接口|`interface`。在后文会详细讨论它们。

- 除了基本数据类型，不提供值类型，对象全部为引用类型。在少量情况（下面会详细讨论），需要将基本数据类型以对象的形式保存，Gloom提供了包装类，并提供了自动装箱和拆箱。

    ```go
    // basic type
    let i1 int = 1;
    // auto boxing
    let i2 Int = 2;
    // auto unboxing
    let i3 int = i1;
    
    println(i1) // 1
    println(i2) // Int(2)
    println(i3) // 1
    
    println([100]) // [100]
    // which is Array<int> 
    println([100,"233"]) // [Int(100), "233"]
    // which in fact is Array<Any> 
    
    let fn1 = func(int i){ println(i) }
    fn1(i2) // i2 is a object of Int type
    
    let fn2 = func(Int i){ println(i) }
    fn2(1111) // 1111 is a value of int type
    ```

#### Expression & Statement

- 变量声明|*variable declare*

    声明时必须初始化。

    ```js
    // Rust风格的let语句
    let variable : Type = Type.factoryFunc()
    // ':'可以被省略
    let variable Type = Type.factoryFunc()
    // 其中Type是可选的，不写也可由类型推导出来
    let variable = Type.factoryFunc()
    ```
    
- 条件控制|*condition control* 

    是表达式*expression*而非语句*statement* ，这意味着有如下用法：

    ```js
    let num = 114514
    let newNum = if(num > 10){
        num * 2
    }else if(num > 5){
        num + 5      
    }else{
        num + 1
    }
    ```

    程序块内最后一个表达式默认被返回。在表达式结尾加上`;`即可表示这是一条结束的语句*statement*，那么这条表达式的结果则会被抛弃*discard*。

- 循环控制|*loop control*

  ```js
  let n = 0;
  while(n < 1000){
      n = n * 2;
  }
  println(n)
  
  let sum int = 0 
  for i in (0,100){ // 左开右闭
      sum += i
  }
  println(sum)
  ```
  
- 迭代循环|*iteration loop*

    还是*expression*而非*statement* 

    ```js
    let array Array<String> = ["xx","yy","zz"];
    for str in array {
        println(str)
    }
    
    for c in "Gloom" {
        println(c)
    }
    // 依次打印 'G' 'l' 'o' 'o' 'm'
    ```

### Function

- 函数声明与调用

    ```go
    //   函数名    参数列表 返回值
    func printInt(int n) int {
        println(n)
        return n
    }
    // 等价于('->'可以省略)
    func printInt(int n) -> int {
        println(n)
        n
    }
    // 因为函数的最后一个表达式将被返回  
  
    // 调用
    let n = printInt(111)
    ```

- 支持匿名函数（或者叫闭包或lambda表达式），函数可以赋值给变量

    ```go
    let printInt = func(int n){
        println(n)
    }
    let printInt2 = printInt;
    //调用
    printInt(111)
    ```

- 支持闭包捕获

    ```go
    let i int = 100;
    let printInt2 = func(int n){
        println(n + i) // i被捕获进闭包
    }
    ```

- 支持立即执行函数

    ```go
    func(int n){
        println(n)
    }(100)
    ```

- 支持高阶函数，函数可以作为参数和返回值

    ```go
    let fn = func(int n1,int n2) -> Func<(int,int)> {
        // 返回一个匿名函数
        return func(int i1,int i2){
            // 将 n1 n2 捕获进来
            println([n1,n2,i1,i2])
        }
    }(100,223)
    println(fn) 
    // 打印 <nameless>(int,int)->void [100, 223]
    // 其中 [100, 223] 是捕获的值
    fn(11,22) 
    // 打印 [100, 223, 11, 22]
    
    // 上面可以改写成如下
    func(int n1,int n2) -> Func<(int,int)> {
        return func(int i1,int i2){
            println([n1,n2,i1,i2])
        }
    }(100,223)(11,22)
    ```

    关于匿名函数的类型约束，剩余规则如下

    ```go
    // Func没有泛型时，不能有参数或返回值
    let fn Func = func(){}
    
    // 当Func仅有一个泛型参数时，该类型作为唯一参数类型，无返回值
    let printInt Func<int> = func(int n){
        println(n)
    }
    
    // 当Func仅有一组泛型参数时，该组类型作为参数类型，无返回值
    let printTwoInt Func<(int,int)> = func(int n1,int n2){
        println([n1,n2])
    }
    
    let max Func<(int,int),int> = func(int n1,int n2) int {
        if(n1 > n2){
            n1
        }else{
            n2
        }
    }
    // 传入两个int值，返回一个含有两个int的元组
    let swapIntFn Func<(int,int),(int,int)> = func(int n1,int n2) (int,int) {
        (n2,n1)
    }
    ```

### Object-Oriented Programing

- 简化OOP，仅有类和接口。
- 单继承，多实现，内置类型不能被继承，因为它们是用Rust而非GloomScript实现的。
- 无构造函数，无重载（静态工厂函数代替）

```rust
class Person{
    String name
 	int age
    pub func new(String name,int age){
        Person{
            name : name,
            age : age
        }
    }
    pub func olderThan(self, Person other) bool {
        self.age > other.age
    }
    func drop(self){
        println([self.name, self.age,"Drop"])
    }
}

func(){
   	let person = Person.new("haha",11)
    println(person)
}()
// 打印结果：
// Person{ name : "haha", age : 11 }
// ["haha", 11, "Drop"]
```

