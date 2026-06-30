fun main(args:Array<String>) {

    val arr = arrayOf(1, 200, 3, 5)
    println("Smallest Missing Integer is ${findLowestPositiveNumber(arr)}")
}

fun findLowestPositiveNumber(arr: Array<Int>): Int {
    var pointer = arr.size - 1

    var index = 0

    while (index < arr.size && index < pointer) {
        if (arr[index] <=0) {
            val temp = arr[index]
            arr[index] = arr[pointer]
            arr[pointer] = temp
            pointer--
        } else {
            ++index
        }
    }
    arr.map { print("$it ") }; println()


    println("Last Index $pointer")

    for (i in 0..pointer) {
        val locationToUpdate = Math.abs(arr[i] - 1)

        if (locationToUpdate > pointer) continue



        val value = arr[locationToUpdate]
        arr[locationToUpdate] = value * -1

    }
    arr.map { print("$it ") }; println()

    for (i in 0..pointer) {
        if (arr[i] < 0) continue

        return i + 1
    }

    return pointer + 2
}
