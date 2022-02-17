interface Person{
    func speak(self) -> String
    func work(self)
}

class Worker impl Person{
    pub String name
    pub int age
    pub func speak(self) -> String{
        "无产阶级的朴素语言"
    }
    pub func work(self){
        println("劳动最光荣")
    }
}

class Teacher : Worker{
    pub func speak(self) -> String{
        "可能有点啰嗦"
    }
    pub func work(self){
        println("上课好麻烦")
    }
}

func showPerson(Person person){
    println(person.speak())
    person.work()
    println(person)
}

showPerson(Worker{ name : "worker", age : 18 })
showPerson(Teacher{ name : "teacher", age : 23 })

func showWorker(Worker worker){
    println(worker.speak())
    worker.work()
    println(worker)
}

showWorker(Teacher{ name : "another teacher", age : 25 })