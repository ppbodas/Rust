// Write test cases for calculator class
package test

import Calculator
import kotlin.test.Test
import kotlin.test.assertEquals


class CalculatorTest {
    @Test
    fun testAdd() {
        val calculator = Calculator()
        assertEquals(5, calculator.add(2, 3))
    }

    @Test
    fun testSubtract() {
        val calculator = Calculator()
        assertEquals(1, calculator.subtract(3, 2))
    }

    @Test
    fun testMultiply() {
        val calculator = Calculator()
        assertEquals(6, calculator.multiply(2, 3))
    }

    @Test
    fun testDivide() {
        val calculator = Calculator()
        assertEquals(2, calculator.divide(4, 2))
    }

    // Write add test with null pointer exception
    @Test
    fun testAddWithNull() {
        val calculator = Calculator()
        // Get null pointer exception
        try {
            calculator.add(null, 3)
            assert(false)
        } catch (e: NullPointerException) {
            println("Null pointer exception caught")
        }
    }

    // Write subtract test with null pointer exception
    @Test
    fun testSubtractWithNull() {
        val calculator = Calculator()
        // Get null pointer exception
        try {
            calculator.subtract(3, null)
            assert(false)
        } catch (e: NullPointerException) {
            println("Null pointer exception caught")
        }
    }

}