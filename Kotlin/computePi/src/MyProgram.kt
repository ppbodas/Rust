fun main(args:Array<String>) {
    println("Hello World")
    // Generate random number greater than 10000 and less than 20000
    //val iterations = (Math.random() * 10000 + 10000).toInt()
    val iterations = 50000
    println("Number of iterations: $iterations")

    val pi = computePi(iterations)

    println("Computed PI: $pi")
}

fun computePi(iterations: Int): Double {
    var inside = 0
    for (i in 0..<iterations) {
        val x = Math.random()
        val y = Math.random()
        if (x*x + y*y <= 1.0) inside++
        println("Iteration $i Pi = ${(inside * 4.0 / iterations)}")
    }
    return (inside * 4.0 / iterations)
}
