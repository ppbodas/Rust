import java.util.*


data class Player(val firstName: String, val lastName: String, val age: Int)



fun main(args: Array<String>) {
    println("Hello World")
    forLinkedList()
    forArrayList()
    forMutableArrayList()
    forArray()
    forQueue()
    forArrayDeque()
    forStack()
    forMap()
    forHashSet()
    forPriorityQueue()
    forPriorityQueueWithCustomComparator()
    forPriorityQueueWithCustomMultiComparator()
    // 2D Array
    for2DArray()
    for2DArray2()
}

fun forPriorityQueueWithCustomComparator() {
    val container = PriorityQueue<Player>(compareBy<Player> { it.lastName }.reversed())
    println("*".repeat(20))
    println("DataStructure Used for Priority Queue: ${container::class.java.simpleName}")
    container.add(Player("John", "Adams", 30))
    container.add(Player("Jane", "Doe", 25))
    container.add(Player("Bob", "Smith", 35))

    println(container.poll())
    println(container.poll())
    println(container.poll())

    println()
}

fun forPriorityQueueWithCustomMultiComparator() {
    println("Inside forPriorityQueueWithCustomMultiComparator")
    val container = PriorityQueue<Player>(compareBy<Player> { it.lastName }.thenBy { it.firstName })
    println("*".repeat(20))
    println("DataStructure Used for Priority Queue: ${container::class.java.simpleName}")
    container.add(Player("John", "AB", 30))
    container.add(Player("John", "Adams", 30))
    container.add(Player("Jane", "Adams", 25))
    container.add(Player("Bob", "Adams", 35))

    println(container.poll())
    println(container.poll())
    println(container.poll())
    println(container.poll())

    println()
}

fun forPriorityQueue() {

    val container = PriorityQueue<Int>()
    println("*".repeat(20))
    println("DataStructure Used for Priority Queue: ${container::class.java.simpleName}")
    container.add(110)
    container.add(210)
    container.add(310)

    container.map { print("$it ") }
    println()
}

// Create Linked List
fun forLinkedList() {
    val container = LinkedList<Int>()
    println("*".repeat(20))
    println("DataStructure Used for Linked List: ${container::class.java.simpleName}")
    container.add(10)
    container.add(20)
    container.add(30)

    container.map { print("$it ") }
    println()
}

fun forArrayList() {
    val container = listOf(50, 60, 70)
    println("*".repeat(20))
    println("DataStructure Used for Array List: ${container::class.java.simpleName}")
    container.map { print("$it ") }
    println()
}

fun forMutableArrayList() {
    val container = mutableListOf(50, 60, 70)
    container.add(80)
    println("*".repeat(20))
    println("DataStructure Used For Mutable Array1: ${container::class.java.simpleName}")
    container.map { print("$it ") }
    println()

    for (i in container) {  //50 60 70 80
        print("$i ")
    }
    println()

    for (i in 0.. container.lastIndex) {  //50 60 70 80
        print("${container[i] } ")
    }
    println()

    for (i in container.lastIndex downTo 0) {  //80 70 60 50
        print("${container[i] } ")
    }
    println()

    for ((index, value) in container.withIndex()) {  //50 60 70 80
        print("${container[index] } ")
    }
    println()

    for (i in container.indices) {  //50 60 70 80
        print("$i -> ${container[i] } ")
    }

    println()
    for (i in 0 until container.size) {  //50 60 70
        print("$i ${container[i] } ")
    }
    println()

    container.forEach { print("$it ") } //50 60 70 80
    println()

    container.forEachIndexed { index, value -> print("$index -> $value ") } //0 50 1 60 2 70 3 80
    println()
}

fun forArray() {
    val container = arrayOf(51, 61, 71)
    println("*".repeat(20))
    println("DataStructure Used for Array: ${container::class.java.simpleName}")
    container.map { print("$it ") }
    println()
}

fun forQueue() {
    val container = LinkedList<Int>()
    println("*".repeat(20))
    println("DataStructure Used for Queue: ${container::class.java.simpleName}")
    container.add(101)
    container.add(201)
    container.add(301)

    container.map { print("$it ") }
    println()
    container.removeFirst()
    container.map { print("$it ") }
    println()
}


fun forArrayDeque() {
    val container = ArrayDeque<Int>()
    println("*".repeat(20))
    println("DataStructure Used for Kotlin Queue: ${container::class.java.simpleName}")
    container.add(110)
    container.add(210)
    container.add(310)
    container.map { print("$it ") }
    println()

    println(container.first)
    println(container.last)

    container.removeFirst()
    container.removeLast()


    println()
}

fun forStack() {
    val container = Stack<Int>()
    println("*".repeat(20))
    println("DataStructure Used for Stack: ${container::class.java.simpleName}")
    container.push(501)
    container.push(601)
    container.push(701)

    container.map { print("$it ") }
    println()
    container.pop()
    container.map { print("$it ") }
    println()
}

fun forMap() {
    val container = hashMapOf(1 to "A", 2 to "B", 3 to "C")
    println("*".repeat(20))
    println("DataStructure Used for Map: ${container::class.java.simpleName}")
    container.map { print("$it ") }
    println()

    for ((k, v) in container) {
        println("Key: $k, Value: $v")
    }
    println()
}

fun forHashSet() {
    val container = hashSetOf(1, 2, 3)
    println("*".repeat(20))
    println("DataStructure Used for Set: ${container::class.java.simpleName}")
    container.map { print("$it ") }
    println()
}

fun for2DArray() {
    val container = arrayOf(intArrayOf(1, 2, 3), intArrayOf(4, 5, 6))
    println("*".repeat(20))
    println("DataStructure Used for 2D Array: ${container::class.java.simpleName}")
    container.map { it.map { print("$it ") }; println() }

    println(container.get(0).get(2))
}

fun for2DArray2() {
    val row = 4
    val col = 3
    val container = Array(row) {Array(col){1} }
    println("*".repeat(20))
    println("DataStructure Used for 2D Array: ${container::class.java.simpleName}")
    container.map { it.map { print("$it ") }; println() }

    println(container.get(0).get(2))
}
