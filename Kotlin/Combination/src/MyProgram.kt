fun main(args:Array<String>) {
    val arr = mutableListOf(1,2,3,4,5,6)

    print(arr)
    println()

    val k = 4
    println("Generate all combinations")
    printCobinations(arr, mutableListOf(), 0, k)

    println("Generate all permutations")
    printPermutations(arr, 0)
}

// k is the number of elements to be chosen
// out is the output array
// start is the starting index of the input array
fun printCobinations(arr: List<Int>, out: MutableList<Int>, start: Int, k: Int) {
    if (out.size == k) {
        println(out)
        return
    }

    for (i in start until arr.size) {
        out.add(arr[i])
        printCobinations(arr, out, i+1, k)
        out.removeAt(out.size - 1)
    }
}

fun printPermutations(arr: MutableList<Int>, start: Int) {
    if (start == arr.size - 1) {
        println(arr)
        return
    }

    for (i in start until arr.size) {
        swap(arr, start, i)
        printPermutations(arr, start+1)
        swap(arr, start, i)
    }
}

fun swap(arr: MutableList<Int>, index: Int, i: Int) {
    val temp = arr[index]
    arr[index] = arr[i]
    arr[i] = temp
}
