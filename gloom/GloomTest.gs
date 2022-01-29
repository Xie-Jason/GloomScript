
class Test {
    pub String name
    int age
    pub func max(int n1,int n2) int {
        n1 -- ;
        if(n1 > n2){
            n1
        }else{
            n2
        }
    }
    func biggerThan(self,int n1,int n2) bool {
        n1 > n2
    }
    func display(self,int level){
        if(self.biggerThan(11,22)){
            level ++;
        }else{
            level --;
        }
    }
}

interface Compare {
    func compare1(int n1,int n2) bool
    func compare2(num n1,num n2) bool
}

enum State{
    Running(int)
    Stop((String,int))

    pub func print(self,String prefix){
        let arr = [prefix,self];
        println(arr);
    }
}

let n int = 100;
let n1 = 100 + 10 * 2;

println([n,n1])

let str String = "hello"
str.append("666");
String.append(str," 223");
println(str);
func(){
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