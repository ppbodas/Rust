import java.util.*

typealias Point = Pair<Int, Int>

fun main() {

    val p1 = Point(3, 5)

    val m = mutableSetOf<Point>()

    m.add(p1)
    val p2 = Point(3, 5)
    // m.add(p2)


    println(m.size) // Prints 1

    if (m.contains(p2)) {
        println("contains p2")
    }


    val (x, y) = p1

    println(x)

}