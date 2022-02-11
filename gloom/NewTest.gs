let n = 50;

while n > -10 {
    n --
    if n == -5 {
        break
    }
}
println(n);

for i in (0,10,2){
    println(i);
}

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
        println(["stop at ->",item])
        break
    }
}
println("======")

for ch in "GloomScript真不错"{
    println(ch);
    n ++
    if n == 0 {
        break
    }
}
