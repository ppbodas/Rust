import kotlin.system.exitProcess

fun main(args:Array<String>) {
    println("I can do addition subtraction multiplication and division")

    println("What do you want to do? ")
    println("1. Addition")
    println("2. Subtraction")
    println("3. Multiplication")
    println("4. Division")

    println("5. Exit")

    println("Enter your choice: ")
    var choice = readln().toInt()
    println("Enter the first number: ")
    var num1 = readln().toDouble()
    println("Enter the second number: ")
    var num2 = readln().toDouble()
    var result = 0.0
    when(choice){
        1 -> result = num1 + num2
        2 -> result = num1 - num2
        3 -> result = num1 * num2
        4 -> result = num1 / num2
        5 -> exitProcess(0)
        else -> println("Invalid choice")
    }

    // Show result with 2 decimal places without scientific notation
    println("Result: " + String.format("%.2f", result))





}