fun main(args:Array<String>) {
    println(sayNubmer(10))
}

fun sayNubmer(n: Int): String {
    if (1 == n) {
        return "1"
    }
    val result = sayNubmer(n-1)

    return squash(result)
}

fun squash(result: String): String {
    var out = ""
    var previousChar:Char? = result[0]
    var currentCount = 1
    for (i in 1..(result.length-1)) {
        when (result[i] == previousChar) {
            true -> currentCount = currentCount + 1
            false -> {
                out = out + currentCount.toString() + previousChar.toString()
                currentCount = 1
            }
        }
        previousChar = result[i]
    }
    out = out + currentCount.toString() + previousChar.toString()
    return out
}
