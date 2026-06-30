class Node {
     private var data: Int
     var next: Node?
     constructor(data: Int) {
        println("Inside constructor")
        this.data = data
        this.next = null
    }

    constructor(data: Int, message: String) {
        println("Msg: $message")
        this.data = data
        this.next = null
    }

    init {
        println("Inside init")
    }

    // write getter for data
    fun getData(): Int {
        return data
    }


}