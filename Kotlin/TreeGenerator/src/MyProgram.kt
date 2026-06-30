fun main() {
    println("Hello World")

    var n = 10
    val output = generateTrees(1..n)

    println(output)

}

class TreeNode(var value: Int) {
    var left: TreeNode? = null
    var right: TreeNode? = null
}

fun generateTrees(range: IntRange): List<TreeNode> {

    if (range.isEmpty()) return mutableListOf<TreeNode>()

    if (range.first == range.last) return mutableListOf(TreeNode(range.first))

    if (range.first + 1 == range.last) {
        var node1 = TreeNode(range.first)
        var node2 = TreeNode(range.last)
        node1.right = node2

        var node3 = TreeNode(range.first)
        var node4 = TreeNode(range.last)
        node4.left = node3

        return mutableListOf(
            node1, node4
        )
    }
    var output = mutableListOf<TreeNode>()

    for (i in range) {
        val left = generateTrees(range.first until i)
        val right = generateTrees(i+1 until  range.last + 1)

        for (l in left) {
            val tree = mutableListOf<TreeNode>()
            for (r in right) {
                var node = TreeNode(i)
                node.left = l
                node.right = r

                output.add(node)

            }
        }
    }

    return output
}