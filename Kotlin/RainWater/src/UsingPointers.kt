class UsingPointers {

    fun trappedWater(inputArray: Array<Int>) : Int {
        var left = 0
        var right = inputArray.size - 1
        var leftMax = 0
        var rightMax = 0

        var trappedWater = 0
        while (left <= right) {
            if (rightMax <= leftMax) {
            // This means as rightmax is already less than leftmax,
            // trapped water will only depend on right value and right max
                trappedWater += Math.max(0, rightMax - inputArray[right])
                rightMax = Math.max(rightMax, inputArray[right])
                right -= 1
            } else {
                trappedWater += Math.max(0, leftMax- inputArray[left])
                leftMax = Math.max(leftMax, inputArray[left])
                left += 1
            }
        }
        return trappedWater
    }
}