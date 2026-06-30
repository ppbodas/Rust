fun main(args:Array<String>) {
    println("Hello World")

    val s = "geeksforgeeks"
    val t = "ork"

    // s = "ADOBECODEBANC", t = "ABC"


    val tMap: Map<Char, Int> = t.convertStringToMap()

    if (s.length < t.length) return

    var left = 0
    var right = t.length

    var minLeft = 0
    var minRight = Integer.MAX_VALUE

    // ToDo: One more optimization can be done to not compute map again and again
    while (left <= s.length - t.length && right <= s.length) { // There is nothing fancy here. Just max indexes it can go logically
        println(s.substring(left, right))
        var sMap = s.substring(left, right).convertStringToMap()

        while (checkMaps(tMap, sMap)) {
            if (right - left < minRight - minLeft) {
                minLeft = left
                minRight = right
            }
            left++
            sMap = s.substring(left, right).convertStringToMap()
        }
        right++
    }

    if (minRight != Integer.MAX_VALUE) {
        println("Min Left is $minLeft Min right is $minRight")

        println("Final output is ${s.substring(minLeft, minRight)}")
    } else {
        println("No substring exist")
    }



}

fun checkMaps(baseMap: Map<Char, Int>, otherMap: Map<Char, Int>) : Boolean {
    for (pair in baseMap) {
        if (!otherMap.containsKey(pair.key)) return false

        if (otherMap[pair.key]!! < baseMap[pair.key]!!) return false
    }
    return true
}

fun String.convertStringToMap(): Map<Char, Int> {
    val map = HashMap<Char, Int>()
    for (ch in this) {
        map[ch] = map.getOrDefault(ch, 0) + 1
    }
    return map
}
