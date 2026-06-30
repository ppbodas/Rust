import java.util.Stack

fun main(args:Array<String>) {
    println("Hello World")

    val arr = arrayOf(1,1)

    println(findLargestArea(arr))
}

fun findLargestArea(heights: Array<Int>): Int {
    if (heights.size == 1) return heights[0]
    var stack = Stack<Int>()

    var globalMax = Int.MIN_VALUE
    for (i in heights.indices) {
        if (stack.isEmpty() || heights[stack.peek()] < heights[i]) {
            stack.push(i)
        } else {
            while (stack.isNotEmpty() && heights[stack.peek()] >= heights[i] ) {
                val index = stack.pop()
                val rightBoundary = (i - index - 1)
                val leftBoundary = if (stack.isEmpty()) index + 1 else index - stack.peek()
                globalMax = Math.max(globalMax, (rightBoundary + leftBoundary) * heights[index])
            }
            stack.push(i)
        }
    }

    if (stack.isNotEmpty()) {
        val last = stack.peek()
        while (stack.isNotEmpty()) {
            val index = stack.pop()
            val rightBoundary = last - index
            val leftBoundary = if (stack.isEmpty()) index + 1 else index - stack.peek()
            globalMax = Math.max(globalMax, (rightBoundary + leftBoundary) * heights[index])
        }
    }


    return globalMax

}
