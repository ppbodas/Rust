class XORList {
    var head :  Node? = null
    var tail :  Node? = null

    var map :  HashMap<Int, Node> = HashMap()

    fun add(value : Int) {
        val newNode = Node(value)
        map[System.identityHashCode(newNode)] = newNode
        if (head == null) {
            head = newNode
        } else {
            tail!!.both = tail!!.both.xor(0).xor(System.identityHashCode(newNode))
            newNode.both = System.identityHashCode(tail).xor(0)
        }
        tail = newNode
    }

    private fun findNode(value: Int) : Pair<Node, Int>? {
        var curr = head
        var previousNodeHashCode = 0
        while (curr != null) {
            if (curr.value == value) {
                return Pair(curr, previousNodeHashCode)
            }

            val temp = curr
            curr = map[curr.both.xor(previousNodeHashCode)]
            previousNodeHashCode = System.identityHashCode(temp)
        }
        return null
    }

    fun printList() {
        var curr = head
        var previousNodeHashCode = 0
        while (curr != null) {
            println(curr.value)
            val temp = curr
            curr = map[curr.both.xor(previousNodeHashCode)]
            previousNodeHashCode = System.identityHashCode(temp)
        }
    }
}