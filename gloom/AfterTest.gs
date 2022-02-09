
class Test {
    pub String name
    pub int age
    pub func max(int n1,int n2) int {
        if(n1 >= n2){
            n1
        }else{
            n2
        }
    }
    pub func olderThan(self, Test other) bool {
        self.age > other.age
    }
    func drop(self){
        println([self.name,"Drop"])
    }
}

interface Compare {
    func compare1(int n1,int n2) bool
    func compare2(num n1,num n2) bool
}

enum State{
    Running(int)
    Stop((String,int))

    pub func show(self,String prefix){
        let arr = [prefix,self];
        println(arr);
    }
}

let n int = 100;
let n1 = 100 + 10 * 2;

println([n,n1])

let str String = "hello";
str = str.append("666");
str = String.append(str," 223");
println(str);

func(){
    // n1 are captured here
    println([n1,str])
}()
println(["这里是n->",n])

n = if n > 25 {
    n / 2
}else{
    n * 2
}


let myPrint = func(int i){
    println(["这里是后面的n->",i]);
}
myPrint(n)

println("--------")
func(int i,,,,int n){ println(i) let new = 100 * n + 114514 n = new println(new)  }(n1,10 * 10)
println("--------")


let fn = func(int i,int n) Func<(int,int)> {
    return func(int i1,int i2){
        println([i,n,i1,i2]);
    }
}(100,223)
println(fn)
fn(11,22)

let count = 0;
println([count,n])
while n > 0 {
    n --
    count ++
}
println([count,n])

Func.printBody(Test.olderThan)

let test1 = Test{
    name : "Tester",
    age : 114514
}
println(test1)
let test2 = Test{
    name : "haha",
    age : 18
}

println([test1.age,test2.age,test1.olderThan(test2)])

let forCount = 0;
for i in (0,100,1){
    forCount ++
}
println(forCount)
for i in (0,100,2){
    forCount --
}
println(forCount)

println("======")
for item in [0,5,10,15,20] {
    println(item)
    if item >= 8 {
        Test{ name : "Will be Dropped now", age : 18 }
        let test = Test{ name : "Will be Dropped after if scope", age : 18 }
        println(["stop at ->",item])
        break
    }
}
println("======")

for ch in "GloomScript真不错"{
    println(ch)
}
