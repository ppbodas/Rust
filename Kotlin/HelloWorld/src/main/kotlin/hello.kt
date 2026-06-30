fun main() {
    println("Hello World")

    var point = Triple(0, 2, 3)

    when {
        (point.first == 0 && point.second == 0 && point.third == 0) -> println("Origin")
        point.first == 0 -> println("YZ plane")
        point.second == 0 -> println("XZ plane")
        point.third == 0 -> println("XY plane")
        else -> println("In space")
    }

    println(add(2, 3))

    val x: Int? = null

    println(x.toString())

    println("Reached")



}

fun add(a:Int, b:Int):Int {
    return a+b
}

fun multiply(a:Int, b:Int):Int? {
    return a*b
}