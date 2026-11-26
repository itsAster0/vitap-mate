package vitapmate.features.login

import kotlin.test.Test
import kotlin.test.assertEquals

class GuidePagesTest {
    private fun page(step: Int?, title: String) = GuidePage(step, title, "", null, null, null)

    @Test
    fun pagesForKeepsOrderAndSkipsUntagged() {
        val all = listOf(page(1, "add"), page(null, "old"), page(3, "reload"), page(2, "verify"), page(3, "from"))
        assertEquals(listOf("reload", "from"), pagesFor(3, all).map { it.title })
        assertEquals(listOf("add"), pagesFor(1, all).map { it.title })
        assertEquals(emptyList(), pagesFor(4, all))
    }

    @Test
    fun anOldGuideWithoutStepsShowsEveryPage() {
        val old = listOf(page(null, "add"), page(null, "verify"))
        assert(pagesFor(1, old).isEmpty())
        assertEquals(listOf("add", "verify"), pagesOrAll(1, old).map { it.title })
        val tagged = listOf(page(1, "add"), page(null, "old"))
        assertEquals(listOf("add"), pagesOrAll(1, tagged).map { it.title })
    }
}
