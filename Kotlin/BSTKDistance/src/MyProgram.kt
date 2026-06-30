import java.util.*

data class Node(val v: Int, var left: Node? = null, var right: Node? = null)

class BST(val root: Node) {

    fun insert(v: Int) {
        insert(root, v)
    }

    fun inOrderTraverse() {
        inOrderTraverse(root)
    }

    fun find(v: Int): Node? {
        return find(root, v)
    }

    fun bfs(node: Node?, depth: Int) {
        node?: return

        val q = ArrayDeque<Node>()
        q.add(node)

        var k = 0

        while (q.isNotEmpty()) {
            val size = q.size

            for (i in 0..<size) {
                val node = q.pollFirst()
                if (k == depth) {
                    println(node.v)
                }
                if (null != node.left) {
                    q.add(node.left)
                }
                if (null != node.right) {
                    q.add(node.right)
                }
            }
            k++
        }
    }

    fun dfs(node: Node?, depth: Int) {
        node?: return

        if (depth == 0) println(node.v)

        dfs(node.left, depth - 1)
        dfs(node.right, depth - 1)

    }

    private fun insert(root: Node, v: Int) {
        if (v < root.v) {
            if (null == root.left) {
                val node = Node(v)
                root.left = node
                return
            }
            insert(root.left!!, v)
        } else {
            if (null == root.right) {
                val node = Node(v)
                root.right = node
                return
            }
            insert(root.right!!, v)
        }
    }


    private fun inOrderTraverse(root: Node?) {
        root?: return

        inOrderTraverse(root.left)
        println(root.v)
        inOrderTraverse(root.right)
    }

    private fun find(root: Node?, v: Int): Node? {
        root?: return null

        if (root.v == v) return root

        if (v < root.v) {
            return find(root.left, v)
        } else {
            return find(root.right, v)
        }
    }

}

fun main() {

    val bst = BST(Node(100))

    constructTree(bst)


    // bst.inOrderTraverse()

    val node = bst.find(200)

    // bst.bfs(node, 2)

    bst.dfs(node, 3)

}

fun constructTree(bst: BST) {
    bst.insert(50)
    bst.insert(25)
    bst.insert(60)
    bst.insert(200)
    bst.insert(150)
    bst.insert(300)
    bst.insert(250)
    bst.insert(350)
    bst.insert(325)
    bst.insert(400)
}