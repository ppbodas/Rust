fun main(args:Array<String>) {
    // Create 2D int array of 4 by 4 with value 0, 1, 2...
    var arr = Array(3) {IntArray(4) }

    fill2DArray(arr)

    // print 2d array
    print2DArray(arr)

    //print2DArrayInZigZagForm(arr)
    print2DArrayInZigZagDiagonalFormat(arr)

}

fun print2DArrayInZigZagForm(arr: Array<IntArray>) {
    var col = 0
    var row = 0

    val size = arr.size

    var up = true

    while (row >= 0 && row < size && col >= 0 && col < size) {
        if (up){
            while (row >= 0  && col < size){
                print(arr[row][col])
                print(" ")
                if (row == 0) {
                    if (col == size - 1) {
                        row++
                        break
                    } else {
                        col++
                        break;
                    }
                } else if (col == size - 1) {
                    row++
                    break;
                } else {
                    row--
                    col++
                }
            }
        } else {
            while (row < size && col < size){
                print(arr[row][col])
                print(" ")
                if (col == 0) {
                    if (row == size - 1) {
                        col++ // move to next column
                        break
                    } else {
                        row++
                        break
                    }
                } else if (row == size -1) {
                    col++ // move to next column
                }
                else {
                    row++
                    col--
                }
            }
        }
        up = !up
    }
}

private fun fill2DArray(arr: Array<IntArray>) {
    // fill 2d array
    var count = 0
    for (i in 0 until arr.size) {
        for (j in 0 until arr[0].size) {
            arr[i][j] = count++
        }
    }
}

private fun print2DArray(arr: Array<IntArray>) {

    for (i in 0 until arr.size) {
        for (j in 0 until arr[0].size) {
            print("${arr[i][j]} ")
        }
        println()
    }
}