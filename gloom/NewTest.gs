import "./AfterTest.gs"

let a : num = 1.1
println(a)

let forCount : int = 0;
for i in (0,100,1){
    forCount ++
}
println(forCount)
for i in (0,100,2){
    forCount --
}
println(forCount)

let n = 10;
let count = 0;
println([count,n])
while n > 0 {
    n --
    count ++
}
print(count)
println(n)
println(input())