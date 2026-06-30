import com.sun.source.tree.Tree
import kotlin.random.Random

data class TreeNode(var left: TreeNode?, var right: TreeNode?, val value: Int = 0)

var prev : TreeNode? = null
var head : TreeNode? = null

class BinaryTree(var root: TreeNode) {
    fun addNode(value: Int) {
        addNode(root, value)
    }

    fun inOrderTraversal() {
        inOrderTraversal(root)
    }

    fun bst2dll() : TreeNode? {
        prev = null
        head = null
        bst2dll(root)
        return head
    }

    private fun bst2dll(root: TreeNode?) {
        root ?: return

        bst2dll(root.left)

        if (null == prev) {
            head = root
        } else {
            root.left = prev
            prev!!.right = root
        }
        prev = root

        bst2dll(root.right)


        return
    }

    private fun inOrderTraversal(node: TreeNode?) {
        node ?: return

        inOrderTraversal(node.left)
        println(node.value)
        inOrderTraversal(node.right)
    }

    private fun addNode(cur: TreeNode, value: Int) {
        if (cur.value > value) {
            if (null == cur.left) {
                cur.left = TreeNode(null, null, value)
            } else {
                addNode(cur.left!!, value)
            }
        } else {
            if (null == cur.right) {
                cur.right = TreeNode(null, null, value)
            } else {
                addNode(cur.right!!, value)
            }
        }
    }
}

fun main(args: Array<String>) {
    val bt = constructBinaryTree()
    // bt.inOrderTraversal()

    var head = bt.bst2dll()

    while (head != null) {
        println(head.value)
        head = head.right
    }
    println("Last node value ${prev!!.value}")  // To make circular link list just connect head and prev
}

fun constructBinaryTree(): BinaryTree {
    val root = TreeNode(null, null, 100)
    val bt = BinaryTree(root)

    for (i in 0..8) {
        bt.addNode(Random.nextInt(0, 200))
    }

    return bt
}
