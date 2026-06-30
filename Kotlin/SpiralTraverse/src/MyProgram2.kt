fun print2DArrayInZigZagDiagonalFormat(arr: Array<IntArray>) {
    val rowCount = arr.size
    val colCount = arr[0].size

    println("Max Rows $rowCount Max Cols $colCount")

    // byAmznQ(rowCount, colCount, arr)
    var up  = true
    for (i in 0 until rowCount + colCount - 1) {
        //println("i $i")
        if (up) {
            var row = i
            var col = 0

            if (i > rowCount -1 ) {
                row = rowCount -1
                col = i - row // Whatever extra rows we add to col
            }
            while (row >= 0 && col < colCount) {
                print("${arr[row][col]} ")
                row--
                col++
            }
        } else {
            var col = i
            var row = 0

            if (i > colCount -1 ) {
                col = colCount -1
                row = i - col   // Whatever extra cols we add to row
            }
            while (col >= 0 && row < rowCount) {
                print("${arr[row][col]} ")
                row++
                col--
            }
        }
        up = !up
        println()
    }
}

private fun byAmznQ(rowCount: Int, colCount: Int, arr: Array<IntArray>) {
    for (i in 0 until rowCount + colCount - 1) {
        if (i % 2 == 0) {
            for (j in Math.min(i, colCount - 1) downTo 0) {
                val k = i - j
                if (k < rowCount) {
                    print("${arr[k][j]} ")
                }
            }
        } else {
            for (j in Math.min(i, rowCount - 1) downTo 0) {
                val k = i - j
                if (k < colCount) {
                    print("${arr[j][k]} ")
                }
            }
        }
        println()
    }
}