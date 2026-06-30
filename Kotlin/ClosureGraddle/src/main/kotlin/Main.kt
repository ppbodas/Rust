import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    var job = launch {
        println(Thread.currentThread().name)
    }

    job.join()
}