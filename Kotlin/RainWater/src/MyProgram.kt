fun main(args:Array<String>) {
    println("Hello World")

    val inputArray  = arrayOf(3, 0, 1, 0, 4, 0, 2)
    val inputArray1  = arrayOf(3, 0, 2, 0, 4)

    inputArray.map { print("$it ") }
    println()

    println("Using array method ${UsingArray().trappedWater(inputArray)}")
    println("Using 2 Pointer method ${UsingPointers().trappedWater(inputArray)}")
}