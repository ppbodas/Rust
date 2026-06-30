
data class Node(val data : Int = 0, var next : Node? = null)

fun main(args:Array<String>) {
    println("Hello World")

    // Create single linked list and return head
    var head = createLinkedList(11)

    // print linked list
    printLinkedList(head)

    val k = 3

    // reverse linked list
    head = reverseLinkedList(head, k)

    // print linked list
    printLinkedList(head)

}
// Reverse linked list in group of k
// If k nodes are not present don't reverse
fun reverseLinkedList(head: Node?, k: Int): Node? {
    var nextNode = head
    var count = 0
    while (count < k) {
        if (nextNode == null) {
            return head
        }
        nextNode = nextNode.next
        count++
    }

    if (count == k) {
        val reverseHead = reverseKGroup(head, k)
        head?.next = reverseLinkedList(nextNode, k)
        return reverseHead
    }

    // println("NextNode data ${nextNode!!.data}")


    return head
}

fun reverseKGroup(head: Node?, k: Int): Node? {
    var cur = head?.next
    var prev = head

    var count = 1
    while (count < k) {
        val temp = cur?.next
        cur?.next = prev
        prev = cur
        cur = temp
        count++
    }
    return prev
}

fun printLinkedList(head: Node?) {
    var temp = head
    while (temp != null) {
        print("${temp.data} ")
        temp = temp.next
    }
    println()
}

fun createLinkedList(count: Int): Node? {
    var head: Node? = null
    var temp: Node? = null
    for (i in 1..count) {
        if (head == null) {
            head = Node(i)
            temp = head
        } else {
            temp?.next = Node(i)
            temp = temp?.next
        }
    }
    return head
}
