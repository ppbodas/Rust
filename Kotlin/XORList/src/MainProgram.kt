fun main() {

    println("Hello World")
    var xorList : XORList = XORList()
    xorList.add(1)
    xorList.add(2)
    xorList.add(3)
    xorList.add(4)

    xorList.printList()

    xorList.delete(2)

    xorList.printList()

}