let n = 50;
#  0 LoadDirectInt(50)
#  1 WriteLocalInt(0, 0)


let n1 = 100 + 10 * 2;
#  2 LoadDirectInt(100)
#  3 LoadDirectInt(10)
#  4 LoadDirectInt(2)
#    [100, 10, 2]
#  5 Mul
#    [100, 20]
#  6 Plus
#    [120]
#  7 WriteLocalInt(0, 1)
#    []

let n2 = n1 + 10 * 2 + 20 / 2 * 5;
#  8 ReadLocal(0, 1)
#  9 LoadDirectInt(10)
# 10 LoadDirectInt(2)
#    [120, 10, 2]
# 11 Mul
#    [120, 20]
# 12 Plus
#    [140]
# 13 LoadDirectInt(20)
# 14 LoadDirectInt(2)
#    [140, 20, 2]
# 15 Div
#    [140, 10]
# 16 LoadDirectInt(5)
#    [140, 10, 5]
# 17 Mul
#    [140, 50]
# 18 Plus
#    [190]
# 19 WriteLocalInt(1, 0)


println([n1,n2])
# 20 LoadDirectDefFn(0)
#    [println(Any)->void]
# 21 ReadLocal(1, 0)
#    [println(Any)->void, 190]
# 22 ReadLocal(0, 1)
#    [println(Any)->void, 190, 120]
# 23 CollectArray(int, 2)
#    [println(Any)->void, [120, 190]]
# 24 CallTopFn { nargs: 1 }
#    [none]
# 25 Pop
#    []

while(n > 40){
    n --
}
# 26 ReadLocal(0, 0)
#    [50]
# 27 LoadDirectInt(40)
#    [50, 40]
# 28 GreaterThan
#    [true]
# 29 JumpIfNot(34)
#    []
# 30 ReadLocal(0, 0)
#    [50]
# 31 SubOne
#    [49]
# 32 WriteLocalInt(0, 0)
#    []
# 33 Jump(26)

n = if n > 10 {
    n - 5
}else{
    n + 10
}
// if branch
# 34 ReadLocal(0, 0)
# 35 LoadDirectInt(10)
#    [40, 10]
# 36 GreaterThan
#    [true]
# 37 JumpIfNot(42)
#    []
# 38 ReadLocal(0, 0)
#    [40]
# 39 LoadDirectInt(5)
#    [40, 5]
# 40 Sub
#    [35]
# 41 Jump(47)
// else branch
# 42 LoadDirectBool(true)
# 43 JumpIfNot(47)
# 44 ReadLocal(0, 0)
# 45 LoadDirectInt(10)
# 46 Plus
# 47 WriteLocalInt(0, 0)