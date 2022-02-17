pub static counter int = 0;
for i in (0,100){
    counter ++;
}

for i in (0,5){
    func(){
        counter++;
        println(counter)
    }()
}

func get_counter() -> int{
    static counter = 0;
    counter ++;
    counter
}

for i in (0,5){
    println(get_counter())
}