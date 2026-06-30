@OptIn(ExperimentalStdlibApi::class)
fun main() {
    println("Hello World")

    val x = 12

    println(Integer.toHexString(x))
    println("0001".hexToInt())

    var j = 1

    val l = mutableListOf<Int>()

    for (i in 1..32) {
        if (x and j > 0) {
            l.add(1)
        } else {
            l.add(0)
        }
        j = j shl 1
    }

    l.reversed().map {print(it)}

    val m = binaryRepresentation(3456)
    println()

    m.map {print(it)}

    println(Integer.toBinaryString(3456))

    println("a".hexToInt().inv())


}


fun binaryRepresentation(x: Int) : List<Int> {
    val l = mutableListOf<Int>()
    var j = 1
    for (i in 0..31) {
        if ((x and j) > 0) {
            l.add(1)
        } else {
            l.add(0)
        }
        j = j.shl(1)
    }

    return l.reversed()
}
