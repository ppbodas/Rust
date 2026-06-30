import kotlin.math.abs

fun main(args:Array<String>) {
    println("Hello World")
    val s = "dog cat cat fish dog elephant"
    println(findDistanceBetweenWords(s.split(" "), "elephant", "fish"))
}

fun findDistanceBetweenWords(words:List<String>, w1:String, w2:String):Int {
    var p1 = -1
    var p2 = -1
    var distance = Integer.MAX_VALUE

    for (iter in words.withIndex()) {
        when(iter.value) {
            w1 -> p1 = iter.index
            w2 -> p2 = iter.index
        }
        if (p1 != -1 && p2 != -1) {
            distance = abs(p1 - p2).coerceAtMost(distance)
        }
    }
    return distance

}