// basic type
let i1 int = 1;
println(i1) // 1

// auto boxing
let i2 Int = 2;
println(i2) // Int(2)

// auto unboxing
let i3 int = i1;
println(i3) // 1

// boxing and un boxing by 'as' operator
let i4 = 10 as Int;
println(i4) // Int(10)
let i5 = i4 as int;
println(i5) // 10

println([100]) // [100]

println([100,"233"]) // [Int(100), "233"]

println((100,"233")) // (100, "233")

let fn1 = func(int i){ println(i) }
fn1(i2) // i2 is a object of Int type

let fn2 = func(Int i){ println(i) }
fn2(1111) // 1111 is a value of int type