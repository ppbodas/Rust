import java.util.LinkedList


fun main(args:Array<String>) {
    println("Hello World")
    val line = readln()

    val split = line.split(',').map { it.strip() }.map { when(it) {
        "null" -> null
        "" -> null
        else -> it.toInt()
    } }

    println(split)

    val root = generateTree(split)
    if (root != null) {
        printTree(root)
    }

    println()

    if (root != null) {
        printZigZagTree(root)
    }

}

fun printZigZagTree(root: TreeNode) {
    val queue = LinkedList<TreeNode>()
    queue.add(root)
    print("${root.value} ")


    var leftToRight = false
    while (queue.isNotEmpty()) {
        val size = queue.size

        val nextLevelStack = LinkedList<TreeNode>()

        for (i in 0 until size) {
            val node = queue.remove()
            if (leftToRight) {
                addLeft(node, nextLevelStack)
                addRight(node, nextLevelStack)
            } else {
                addRight(node, nextLevelStack)
                addLeft(node, nextLevelStack)
            }
        }

        nextLevelStack.forEach { print("${it.value} ") }

        while (nextLevelStack.isNotEmpty()) {
            queue.add(nextLevelStack.removeLast())
        }
        leftToRight = !leftToRight
    }
}

fun printTree(root: TreeNode) {
    val queue = LinkedList<TreeNode>()
    queue.add(root)

    while (queue.isNotEmpty()) {
        val node = queue.remove()
        print("${node.value} ")
        addLeft(node, queue)
        addRight(node, queue)
    }
}

private fun addRight(node: TreeNode, queue: LinkedList<TreeNode>) {
    if (node.right != null) {
        queue.add(node.right!!)
        return
    }
    if (node.value != null) {
        queue.add(TreeNode(null, null, null))
    }
}

private fun addLeft(node: TreeNode, queue: LinkedList<TreeNode>) {
    if (node.left != null) {
        queue.add(node.left!!)
        return
    }
    if (node.value != null) {
        queue.add(TreeNode(null, null, null))
    }
}

// Generate Tree
fun generateTree(list : List<Int?>) : TreeNode? {
    if (list.isEmpty()) return null
    if (list[0] == null) {
        return null
    }
    val root = TreeNode(list[0], null, null)
    val queue = LinkedList<TreeNode>()
    queue.add(root)

    var index = 1

    while (queue.isNotEmpty()) {
        val node = queue.remove()
        val leftValue = list.getOrNull(index++)
        val rightValue = list.getOrNull(index++)

        if (leftValue != null) {
            val leftNode = TreeNode(leftValue, null, null)
            node.left = leftNode
            queue.add(leftNode)
        }

        if (rightValue != null) {
            val rightNode = TreeNode(rightValue, null, null)
            node.right = rightNode
            queue.add(rightNode)
        }
    }

    return root
}



