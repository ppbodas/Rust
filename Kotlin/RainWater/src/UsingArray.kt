class UsingArray() {

    fun trappedWater(inputArray: Array<Int>) : Int {
        val length = inputArray.size
        if (0 == length) {
            print("0 length array input")
            return 0
        }

        var maxLeft:Array<Int> = Array(length){0}
        var maxRight:Array<Int> = Array(length){0}

        maxLeft[0] = 0

        for (i in 0..length-2) { // 0, 1, 2, 3, 4
            maxLeft[i + 1] =  Math.max(maxLeft[i], inputArray[i])
        }

        maxRight[length - 1] = 0
        for (i in length-2 downTo  0) {
            maxRight[i] = Math.max(inputArray[i+1], maxRight[i+1])
        }

        maxLeft.map { print("$it ") }
        println()

        maxRight.map { print("$it ") }
        println()

        var trappedWater = 0
        for (i in 1..length-2) {
            trappedWater += Math.max(0, Math.min(maxLeft[i], maxRight[i]) - inputArray[i])
        }

        return trappedWater

    }
}