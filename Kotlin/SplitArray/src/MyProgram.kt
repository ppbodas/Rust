fun main(args:Array<String>) {
    println("Hello World")

    val arr = arrayOf(5, 1, 2, 7, 3, 4)
    val k = 3

    val maxSum: Int = solve(arr, k)

    println(maxSum)
}

fun solve(arr: Array<Int>, k: Int): Int {
    var start = 0
    var end = 0
    for (i in arr.indices) {
        start = maxOf(start, arr[i])
        end += arr[i]
    }

    var output = -1

    while (start <= end) {
        val mid = (start + end) / 2
        if (isPossible(arr, k, mid)) {
            end = mid - 1
            output = mid
        } else {
            start = mid + 1
        }
    }

    return output

}

fun isPossible(arr: Array<Int>, k: Int, mid: Int): Boolean {
    var count = 1
    var sum = 0

    for (i in arr.indices) {
        if (arr[i] > mid) {
            return false
        }

        sum += arr[i]
        if (sum > mid) {
            count++
            sum = arr[i]
        }
    }

    return count <= k
}
