import java.io.File

fun main(args:Array<String>) {

    // Read File
    println()
    println("*** Read File ***")
    val lines = File("data/input.txt").readLines()
    lines.map { println(it) }

    // Read file line by line
    println()
    println("*** Read file line by line ***")
    File("data/input.txt").forEachLine { println(it) }

    // Read file line by line and split by space
    println()
    println("*** Read file line by line and split by space ***")
    File("data/input.txt").forEachLine {
        val words = it.split(" ")
        words.map { println(it) }
    }


    // Read standard input
    println()
    println("*** Read standard input. Enter to exit ***")
    while (true) {
        val line = readLine()
        if (line.isNullOrEmpty()) break
        println(line)
    }


}

